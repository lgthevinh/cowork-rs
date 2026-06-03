use super::agent_tool::AgentTool;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionMessageToolCall, ChatCompletionMessageToolCallChunk,
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
        ChatCompletionStreamOptions, ChatCompletionTool, ChatCompletionToolType, CompletionUsage,
        CreateChatCompletionRequest, CreateChatCompletionRequestArgs, CreateChatCompletionResponse,
        FinishReason, FunctionCall, FunctionObject,
    },
};
use futures_util::StreamExt;
use std::collections::BTreeMap;
use std::sync::RwLock;

const MAX_TOOL_ITERATIONS: usize = 10;

#[derive(Debug, Clone, Default)]
pub struct AgentResponse {
    pub content: String,
    pub finish_reason: Option<FinishReason>,
    pub usage: Option<CompletionUsage>,
    pub tool_calls: Vec<ChatCompletionMessageToolCallChunk>,
}

#[async_trait::async_trait]
pub trait AgentStreamCallback: Send + Sync {
    async fn on_token(&self, token: &str);
    async fn on_complete(&self, full_text: &str);
    async fn on_error(&self, error: &anyhow::Error);
    async fn on_usage(&self, usage: &CompletionUsage);
    async fn on_tool_start(&self, tool_name: &str, arguments: &str);
    async fn on_tool_result(&self, tool_name: &str, result: &str);
}

pub struct Agent {
    id: String,
    name: String,
    system_instruction: String,
    llm_runtime: RwLock<AgentLlmRuntime>,
    tools: Vec<Box<dyn AgentTool + Send + Sync>>,
}

struct AgentLlmRuntime {
    model: String,
    llm_client: Client<OpenAIConfig>,
}

impl Agent {
    pub fn new(
        id: String,
        name: String,
        system_instruction: String,
        model: String,
        llm_client: Client<OpenAIConfig>,
        tools: Vec<Box<dyn AgentTool + Send + Sync>>,
    ) -> Self {
        Self {
            id,
            name,
            system_instruction,
            llm_runtime: RwLock::new(AgentLlmRuntime { model, llm_client }),
            tools,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn system_instruction(&self) -> &str {
        &self.system_instruction
    }

    pub fn model(&self) -> String {
        self.llm_runtime
            .read()
            .map(|runtime| runtime.model.clone())
            .unwrap_or_else(|_| String::from("unknown"))
    }

    pub fn llm_client(&self) -> anyhow::Result<Client<OpenAIConfig>> {
        Ok(self
            .llm_runtime
            .read()
            .map_err(|_| anyhow::anyhow!("agent llm runtime lock is poisoned"))?
            .llm_client
            .clone())
    }

    pub fn set_llm_config(
        &self,
        model: String,
        llm_client: Client<OpenAIConfig>,
    ) -> anyhow::Result<()> {
        let mut runtime = self
            .llm_runtime
            .write()
            .map_err(|_| anyhow::anyhow!("agent llm runtime lock is poisoned"))?;

        runtime.model = model;
        runtime.llm_client = llm_client;

        Ok(())
    }

    pub async fn call(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> anyhow::Result<CreateChatCompletionResponse> {
        let (llm_client, request) = self.build_request(messages, false)?;

        Ok(llm_client.chat().create(request).await?)
    }

    pub async fn call_stream(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        callback: &dyn AgentStreamCallback,
    ) -> anyhow::Result<String> {
        let response = self.call_stream_response(messages, callback).await?;

        Ok(response.content)
    }

    pub async fn call_stream_response(
        &self,
        mut messages: Vec<ChatCompletionRequestMessage>,
        callback: &dyn AgentStreamCallback,
    ) -> anyhow::Result<AgentResponse> {
        let mut final_response = AgentResponse::default();

        for _ in 0..MAX_TOOL_ITERATIONS {
            let response = self.stream_once(&messages, callback).await?;

            match response.finish_reason {
                Some(FinishReason::ToolCalls) => {
                    let tool_calls = merge_tool_call_chunks(&response.tool_calls);

                    let assistant_msg = ChatCompletionRequestAssistantMessageArgs::default()
                        .tool_calls(tool_calls.clone())
                        .build()?;
                    messages.push(assistant_msg.into());

                    for tc in &tool_calls {
                        let tool_name = &tc.function.name;
                        let args = &tc.function.arguments;

                        callback.on_tool_start(tool_name, args).await;

                        let result = self.execute_tool(tool_name, args).await;
                        let result_content = match result {
                            Ok(output) => output,
                            Err(e) => format!("Error: {e}"),
                        };

                        callback.on_tool_result(tool_name, &result_content).await;

                        let tool_msg = ChatCompletionRequestToolMessageArgs::default()
                            .content(result_content)
                            .tool_call_id(tc.id.clone())
                            .build()?;
                        messages.push(tool_msg.into());
                    }
                }
                _ => {
                    final_response = response;
                    break;
                }
            }
        }

        // Accumulate usage across iterations is already handled since final_response
        // is set to the last response. For multi-iteration, merge usage.
        callback.on_complete(&final_response.content).await;
        Ok(final_response)
    }

    async fn stream_once(
        &self,
        messages: &[ChatCompletionRequestMessage],
        callback: &dyn AgentStreamCallback,
    ) -> anyhow::Result<AgentResponse> {
        let (llm_client, request) = self.build_request(messages.to_vec(), true)?;
        let mut stream = match llm_client.chat().create_stream(request).await {
            Ok(stream) => stream,
            Err(error) => {
                let error = anyhow::Error::from(error).context("failed to create chat stream");
                callback.on_error(&error).await;
                return Err(error);
            }
        };

        let mut response = AgentResponse::default();

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    let error = anyhow::Error::from(error).context("chat stream failed");
                    callback.on_error(&error).await;
                    return Err(error);
                }
            };

            for update in apply_stream_chunk(&mut response, chunk) {
                match update {
                    AgentStreamUpdate::Token(token) => callback.on_token(&token).await,
                    AgentStreamUpdate::Usage(usage) => callback.on_usage(&usage).await,
                }
            }
        }

        Ok(response)
    }

    async fn execute_tool(&self, name: &str, arguments: &str) -> anyhow::Result<String> {
        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == name)
            .ok_or_else(|| anyhow::anyhow!("unknown tool: {name}"))?;
        tool.execute(arguments).await
    }

    fn build_request(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        stream: bool,
    ) -> anyhow::Result<(Client<OpenAIConfig>, CreateChatCompletionRequest)> {
        let runtime = self
            .llm_runtime
            .read()
            .map_err(|_| anyhow::anyhow!("agent llm runtime lock is poisoned"))?;
        let model = runtime.model.clone();
        let llm_client = runtime.llm_client.clone();
        drop(runtime);

        let mut request_messages = Vec::with_capacity(messages.len() + 1);

        request_messages.push(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(self.system_instruction())
                .build()?
                .into(),
        );
        request_messages.extend(messages);

        let mut request = CreateChatCompletionRequestArgs::default();
        request.model(model).messages(request_messages);

        if !self.tools.is_empty() {
            let chat_tools: Vec<ChatCompletionTool> = self
                .tools
                .iter()
                .map(|tool| {
                    let parameters: Option<serde_json::Value> =
                        serde_json::from_str(tool.parameters_json()).ok();
                    ChatCompletionTool {
                        r#type: ChatCompletionToolType::Function,
                        function: FunctionObject {
                            name: tool.name().to_owned(),
                            description: Some(tool.description().to_owned()),
                            parameters,
                            strict: None,
                        },
                    }
                })
                .collect();
            request.tools(chat_tools);
        }

        if stream {
            request
                .stream(true)
                .stream_options(ChatCompletionStreamOptions {
                    include_usage: true,
                });
        }

        Ok((llm_client, request.build()?))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum AgentStreamUpdate {
    Token(String),
    Usage(CompletionUsage),
}

fn apply_stream_chunk(
    response: &mut AgentResponse,
    chunk: async_openai::types::CreateChatCompletionStreamResponse,
) -> Vec<AgentStreamUpdate> {
    let mut updates = Vec::new();

    if let Some(usage) = chunk.usage {
        response.usage = Some(usage.clone());
        updates.push(AgentStreamUpdate::Usage(usage));
    }

    for choice in chunk.choices {
        if let Some(token) = choice.delta.content {
            response.content.push_str(&token);
            updates.push(AgentStreamUpdate::Token(token));
        }

        if let Some(tool_calls) = choice.delta.tool_calls {
            response.tool_calls.extend(tool_calls);
        }

        if choice.finish_reason.is_some() {
            response.finish_reason = choice.finish_reason;
        }
    }

    updates
}

fn merge_tool_call_chunks(
    chunks: &[ChatCompletionMessageToolCallChunk],
) -> Vec<ChatCompletionMessageToolCall> {
    let mut merged: BTreeMap<u32, ChatCompletionMessageToolCall> = BTreeMap::new();

    for chunk in chunks {
        let entry = merged
            .entry(chunk.index)
            .or_insert_with(|| ChatCompletionMessageToolCall {
                id: chunk.id.clone().unwrap_or_default(),
                r#type: chunk
                    .r#type
                    .clone()
                    .unwrap_or(ChatCompletionToolType::Function),
                function: FunctionCall {
                    name: String::new(),
                    arguments: String::new(),
                },
            });

        if let Some(id) = &chunk.id {
            entry.id = id.clone();
        }

        if let Some(func) = &chunk.function {
            if let Some(name) = &func.name {
                entry.function.name.clone_from(name);
            }
            if let Some(args) = &func.arguments {
                entry.function.arguments.push_str(args);
            }
        }
    }

    merged.into_values().collect()
}
