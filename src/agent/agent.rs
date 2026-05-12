use super::agent_tool::AgentTool;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionMessageToolCallChunk, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionStreamOptions, CompletionUsage,
        CreateChatCompletionRequest, CreateChatCompletionRequestArgs, CreateChatCompletionResponse,
        FinishReason,
    },
};
use futures_util::StreamExt;

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
}

pub struct Agent {
    id: String,
    name: String,
    system_instruction: String,
    model: String,
    llm_client: Client<OpenAIConfig>,
    tools: Vec<Box<dyn AgentTool + Send + Sync>>,
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
            model,
            llm_client,
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

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn llm_client(&self) -> &Client<OpenAIConfig> {
        &self.llm_client
    }

    pub async fn call(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> anyhow::Result<CreateChatCompletionResponse> {
        let request = self.build_request(messages, false)?;

        Ok(self.llm_client.chat().create(request).await?)
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
        messages: Vec<ChatCompletionRequestMessage>,
        callback: &dyn AgentStreamCallback,
    ) -> anyhow::Result<AgentResponse> {
        let request = self.build_request(messages, true)?;
        let mut stream = match self.llm_client.chat().create_stream(request).await {
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

        callback.on_complete(&response.content).await;

        Ok(response)
    }

    fn build_request(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        stream: bool,
    ) -> anyhow::Result<CreateChatCompletionRequest> {
        let mut request_messages = Vec::with_capacity(messages.len() + 1);

        request_messages.push(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(self.system_instruction())
                .build()?
                .into(),
        );
        request_messages.extend(messages);

        let mut request = CreateChatCompletionRequestArgs::default();
        request.model(self.model()).messages(request_messages);

        if stream {
            request
                .stream(true)
                .stream_options(ChatCompletionStreamOptions {
                    include_usage: true,
                });
        }

        Ok(request.build()?)
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
