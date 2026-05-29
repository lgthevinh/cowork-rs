use crate::agent::agent_tool::AgentTool;

pub struct GetCurrentTimeTool;

#[async_trait::async_trait]
impl AgentTool for GetCurrentTimeTool {
    fn name(&self) -> &str {
        "get_current_time"
    }

    fn description(&self) -> &str {
        "Get the current date and time in UTC. Returns an ISO 8601 formatted timestamp."
    }

    fn parameters_json(&self) -> &str {
        r#"{"type": "object", "properties": {}, "required": []}"#
    }

    async fn execute(&self, _json_input: &str) -> anyhow::Result<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| anyhow::anyhow!("system time error: {e}"))?;

        let total_secs = now.as_secs();
        let secs_per_day = 86400;
        let days = total_secs / secs_per_day;
        let time_secs = total_secs % secs_per_day;

        let hours = time_secs / 3600;
        let minutes = (time_secs % 3600) / 60;
        let seconds = time_secs % 60;

        let (year, month, day) = days_to_ymd(days);

        Ok(format!(
            "{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02}:{seconds:02} UTC"
        ))
    }
}

fn days_to_ymd(total_days: u64) -> (u64, u64, u64) {
    let remaining = total_days + 719468;
    let era = remaining / 146097;
    let doe = remaining % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
