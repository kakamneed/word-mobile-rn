#[derive(Clone)]
pub struct AiProviderProfile {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub auth_token: String,
}

#[derive(Clone)]
pub struct AiProviderConfig {
    pub primary: AiProviderProfile,
    pub anthropic_fallback: Option<AiProviderProfile>,
    pub backup: AiProviderProfile,
}

pub struct AiAgent {
    config: AiProviderConfig,
}

pub struct ImageInput<'a> {
    pub mime_type: &'a str,
    pub bytes_base64: &'a str,
}

impl AiAgent {
    pub fn new(config: AiProviderConfig) -> Self {
        Self { config }
    }

    pub fn run_ai_passage_json(
        &self,
        system_message: &str,
        user_message: &str,
    ) -> Result<String, String> {
        self.call_anthropic_passage_tool(
            "Primary AI passage",
            &self.config.primary,
            system_message,
            user_message,
        )
        .or_else(|primary_error| {
            self.call_anthropic_fallback_passage_tool(system_message, user_message, &primary_error)
                .or_else(|anthropic_fallback_error| {
                    self.call_backup_passage_tool(
                        system_message,
                        user_message,
                        &primary_error,
                        &anthropic_fallback_error,
                    )
                })
        })
        .or_else(|tool_error| {
            self.run_text_json(system_message, user_message)
                .map_err(|text_error| {
                    format!(
                        "AI passage tool request failed. Tool: {tool_error}. Text fallback: {text_error}"
                    )
                })
        })
    }

    pub fn run_text_json(
        &self,
        system_message: &str,
        user_message: &str,
    ) -> Result<String, String> {
        self.call_anthropic_text(
            "Primary AI",
            &self.config.primary,
            system_message,
            user_message,
        )
        .or_else(|primary_error| {
            self.call_anthropic_fallback_text(system_message, user_message, &primary_error)
                .or_else(|anthropic_fallback_error| {
                    self.call_backup_text(
                        system_message,
                        user_message,
                        &primary_error,
                        &anthropic_fallback_error,
                    )
                })
        })
    }

    pub fn run_image_json(
        &self,
        system_message: &str,
        user_message: &str,
        image: ImageInput<'_>,
    ) -> Result<String, String> {
        self.call_anthropic_image(
            "Primary AI image",
            &self.config.primary,
            system_message,
            user_message,
            &image,
        )
        .or_else(|primary_error| {
            self.call_anthropic_fallback_image(system_message, user_message, &image, &primary_error)
                .or_else(|anthropic_fallback_error| {
                    self.call_backup_image(
                        system_message,
                        user_message,
                        &image,
                        &primary_error,
                        &anthropic_fallback_error,
                    )
                })
        })
    }

    fn anthropic_url(&self, profile: &AiProviderProfile) -> String {
        normalize_provider_url(&profile.base_url, ANTHROPIC_MESSAGES_PATH)
    }

    fn backup_url(&self) -> String {
        normalize_provider_url(&self.config.backup.base_url, OPENAI_RESPONSES_PATH)
    }

    fn call_anthropic_text(
        &self,
        label: &str,
        profile: &AiProviderProfile,
        system_message: &str,
        user_message: &str,
    ) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .map_err(|e| format!("{label} client failed: {e}"))?;
        let url = self.anthropic_url(profile);
        let payload = serde_json::json!({
            "model": &profile.model,
            "max_tokens": 2048,
            "system": system_message,
            "messages": [{ "role": "user", "content": user_message }],
        });
        let response = send_ai_request_with_retry(label, &url, || {
            client
                .post(&url)
                .header("x-api-key", &profile.auth_token)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
        })?;
        let json: serde_json::Value = response
            .json()
            .map_err(|e| format!("{label} response decode failed from {url}: {e}"))?;
        extract_anthropic_text(&json)
            .ok_or_else(|| format!("{label} response missing text content from {url}"))
    }

    fn call_anthropic_passage_tool(
        &self,
        label: &str,
        profile: &AiProviderProfile,
        system_message: &str,
        user_message: &str,
    ) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .map_err(|e| format!("{label} client failed: {e}"))?;
        let url = self.anthropic_url(profile);
        let payload = serde_json::json!({
            "model": &profile.model,
            "max_tokens": 2048,
            "system": system_message,
            "messages": [{ "role": "user", "content": user_message }],
            "tools": [ai_passage_tool_schema_for_anthropic()],
            "tool_choice": { "type": "tool", "name": AI_PASSAGE_TOOL_NAME },
        });
        let response = send_ai_request_with_retry(label, &url, || {
            client
                .post(&url)
                .header("x-api-key", &profile.auth_token)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
        })?;
        let json: serde_json::Value = response
            .json()
            .map_err(|e| format!("{label} response decode failed from {url}: {e}"))?;
        extract_anthropic_tool_input(&json, AI_PASSAGE_TOOL_NAME).ok_or_else(|| {
            format!("{label} response missing {AI_PASSAGE_TOOL_NAME} tool input from {url}")
        })
    }

    fn call_anthropic_image(
        &self,
        label: &str,
        profile: &AiProviderProfile,
        system_message: &str,
        user_message: &str,
        image: &ImageInput<'_>,
    ) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .map_err(|e| format!("{label} client failed: {e}"))?;
        let url = self.anthropic_url(profile);
        let payload = serde_json::json!({
            "model": &profile.model,
            "max_tokens": 2048,
            "system": system_message,
            "messages": [{
                "role": "user",
                "content": [
                    { "type": "text", "text": user_message },
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": image.mime_type,
                            "data": image.bytes_base64
                        }
                    }
                ]
            }],
        });
        let response = send_ai_request_with_retry(label, &url, || {
            client
                .post(&url)
                .header("x-api-key", &profile.auth_token)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
        })?;
        let json: serde_json::Value = response
            .json()
            .map_err(|e| format!("{label} response decode failed from {url}: {e}"))?;
        extract_anthropic_text(&json)
            .ok_or_else(|| format!("{label} response missing text content from {url}"))
    }

    fn call_anthropic_fallback_text(
        &self,
        system_message: &str,
        user_message: &str,
        primary_error: &str,
    ) -> Result<String, String> {
        match &self.config.anthropic_fallback {
            Some(profile) => self
                .call_anthropic_text(
                    "Secondary Anthropic AI",
                    profile,
                    system_message,
                    user_message,
                )
                .map_err(|fallback_error| {
                    format!("Primary: {primary_error}. Secondary Anthropic: {fallback_error}")
                }),
            None => Err(format!(
                "Primary: {primary_error}. Secondary Anthropic: not configured"
            )),
        }
    }

    fn call_anthropic_fallback_passage_tool(
        &self,
        system_message: &str,
        user_message: &str,
        primary_error: &str,
    ) -> Result<String, String> {
        match &self.config.anthropic_fallback {
            Some(profile) => self
                .call_anthropic_passage_tool(
                    "Secondary Anthropic AI passage",
                    profile,
                    system_message,
                    user_message,
                )
                .map_err(|fallback_error| {
                    format!("Primary: {primary_error}. Secondary Anthropic: {fallback_error}")
                }),
            None => Err(format!(
                "Primary: {primary_error}. Secondary Anthropic: not configured"
            )),
        }
    }

    fn call_anthropic_fallback_image(
        &self,
        system_message: &str,
        user_message: &str,
        image: &ImageInput<'_>,
        primary_error: &str,
    ) -> Result<String, String> {
        match &self.config.anthropic_fallback {
            Some(profile) => self
                .call_anthropic_image(
                    "Secondary Anthropic AI image",
                    profile,
                    system_message,
                    user_message,
                    image,
                )
                .map_err(|fallback_error| {
                    format!("Primary: {primary_error}. Secondary Anthropic: {fallback_error}")
                }),
            None => Err(format!(
                "Primary: {primary_error}. Secondary Anthropic: not configured"
            )),
        }
    }

    fn call_backup_text(
        &self,
        system_message: &str,
        user_message: &str,
        primary_error: &str,
        anthropic_fallback_error: &str,
    ) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .map_err(|e| {
                format!(
                    "Backup AI client failed. {anthropic_fallback_error}. Backup client error: {e}"
                )
            })?;
        let url = self.backup_url();
        let payload = serde_json::json!({
            "model": &self.config.backup.model,
            "instructions": system_message,
            "reasoning": { "effort": "low" },
            "input": user_message,
            "store": false
        });
        let response = send_ai_request_with_retry("Backup AI", &url, || {
            client
                .post(&url)
                .header(
                    "Authorization",
                    format!("Bearer {}", self.config.backup.auth_token),
                )
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
        })
        .map_err(|backup_error| {
            format!("All AI providers failed. Primary: {primary_error}. Anthropic fallback: {anthropic_fallback_error}. OpenAI backup: {backup_error}")
        })?;
        let json: serde_json::Value = response
            .json()
            .map_err(|e| format!("Backup AI response decode failed from {url}. Anthropic fallback: {anthropic_fallback_error}. Backup decode error: {e}"))?;
        extract_openai_responses_text(&json).ok_or_else(|| {
            format!("Backup AI response missing output_text from {url}. Anthropic fallback: {anthropic_fallback_error}")
        })
    }

    fn call_backup_passage_tool(
        &self,
        system_message: &str,
        user_message: &str,
        primary_error: &str,
        anthropic_fallback_error: &str,
    ) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .map_err(|e| {
                format!(
                    "Backup AI passage client failed. {anthropic_fallback_error}. Backup client error: {e}"
                )
            })?;
        let url = self.backup_url();
        let payload = serde_json::json!({
            "model": &self.config.backup.model,
            "instructions": system_message,
            "reasoning": { "effort": "low" },
            "input": user_message,
            "tools": [ai_passage_tool_schema_for_openai()],
            "tool_choice": { "type": "function", "name": AI_PASSAGE_TOOL_NAME },
            "store": false
        });
        let response = send_ai_request_with_retry("Backup AI passage", &url, || {
            client
                .post(&url)
                .header(
                    "Authorization",
                    format!("Bearer {}", self.config.backup.auth_token),
                )
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
        })
        .map_err(|backup_error| {
            format!("All AI passage tool providers failed. Primary: {primary_error}. Anthropic fallback: {anthropic_fallback_error}. OpenAI backup: {backup_error}")
        })?;
        let json: serde_json::Value = response.json().map_err(|e| {
            format!("Backup AI passage response decode failed from {url}. Anthropic fallback: {anthropic_fallback_error}. Backup decode error: {e}")
        })?;
        extract_openai_tool_input(&json, AI_PASSAGE_TOOL_NAME).ok_or_else(|| {
            format!("Backup AI passage response missing {AI_PASSAGE_TOOL_NAME} tool input from {url}. Anthropic fallback: {anthropic_fallback_error}")
        })
    }

    fn call_backup_image(
        &self,
        system_message: &str,
        user_message: &str,
        image: &ImageInput<'_>,
        primary_error: &str,
        anthropic_fallback_error: &str,
    ) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .map_err(|e| format!("Backup AI image client failed. {anthropic_fallback_error}. Backup client error: {e}"))?;
        let url = self.backup_url();
        let payload = serde_json::json!({
            "model": &self.config.backup.model,
            "instructions": system_message,
            "reasoning": { "effort": "low" },
            "input": [{
                "role": "user",
                "content": [
                    { "type": "input_text", "text": user_message },
                    {
                        "type": "input_image",
                        "image_url": format!(
                            "data:{};base64,{}",
                            image.mime_type, image.bytes_base64
                        )
                    }
                ]
            }],
            "store": false
        });
        let response = send_ai_request_with_retry("Backup AI image", &url, || {
            client
                .post(&url)
                .header(
                    "Authorization",
                    format!("Bearer {}", self.config.backup.auth_token),
                )
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
        })
        .map_err(|backup_error| {
            format!(
                "All AI image providers failed. Primary: {primary_error}. Anthropic fallback: {anthropic_fallback_error}. OpenAI backup: {backup_error}"
            )
        })?;
        let json: serde_json::Value = response.json().map_err(|e| {
            format!("Backup AI image response decode failed from {url}. Anthropic fallback: {anthropic_fallback_error}. Backup decode error: {e}")
        })?;
        extract_openai_responses_text(&json).ok_or_else(|| {
            format!("Backup AI image response missing output_text from {url}. Anthropic fallback: {anthropic_fallback_error}")
        })
    }
}

fn send_ai_request_with_retry(
    label: &str,
    url: &str,
    mut send: impl FnMut() -> reqwest::Result<reqwest::blocking::Response>,
) -> Result<reqwest::blocking::Response, String> {
    const RETRY_DELAYS_MS: [u64; 2] = [600, 1600];

    for attempt in 0..=RETRY_DELAYS_MS.len() {
        match send() {
            Ok(response) if response.status().is_success() => return Ok(response),
            Ok(response) => {
                let status = response.status();
                let body = response.text().unwrap_or_default();
                let detail = format!(
                    "{label} request failed: HTTP {status} at {url}{}",
                    response_body_suffix(&body)
                );
                if should_retry_status(status) && attempt < RETRY_DELAYS_MS.len() {
                    std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAYS_MS[attempt]));
                    continue;
                }
                return Err(detail);
            }
            Err(error) => {
                let detail = format!("{label} request failed: network error at {url}: {error}");
                if attempt < RETRY_DELAYS_MS.len() {
                    std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAYS_MS[attempt]));
                    continue;
                }
                return Err(detail);
            }
        }
    }

    Err(format!("{label} request failed at {url}"))
}

fn should_retry_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn response_body_suffix(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        let excerpt = trimmed.chars().take(240).collect::<String>();
        format!("; body: {excerpt}")
    }
}

const AI_PASSAGE_TOOL_NAME: &str = "write_ai_passage";

pub const ANTHROPIC_MESSAGES_PATH: &str = "/v1/messages";
pub const OPENAI_RESPONSES_PATH: &str = "/v1/responses";

fn ai_passage_parameters_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "title": {
                "type": "string",
                "description": "A short Chinese title for the passage."
            },
            "paragraphs": {
                "type": "array",
                "minItems": 1,
                "items": {
                    "type": "string"
                },
                "description": "Chinese paragraphs. Put every target word marker inside the paragraph text as [[word:ENTRY_ID]]."
            }
        },
        "required": ["title", "paragraphs"],
        "additionalProperties": false
    })
}

fn ai_passage_tool_schema_for_anthropic() -> serde_json::Value {
    serde_json::json!({
        "name": AI_PASSAGE_TOOL_NAME,
        "description": "Write one Chinese AI reading passage around the supplied wrong words. The tool input is the final passage payload.",
        "input_schema": ai_passage_parameters_schema()
    })
}

fn ai_passage_tool_schema_for_openai() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": AI_PASSAGE_TOOL_NAME,
        "description": "Write one Chinese AI reading passage around the supplied wrong words. The arguments are the final passage payload.",
        "parameters": ai_passage_parameters_schema()
    })
}

pub fn normalize_provider_url(base_url: &str, path_suffix: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with(path_suffix) {
        trimmed.to_string()
    } else {
        format!("{trimmed}{path_suffix}")
    }
}

pub fn base_url_without_suffix(url: &str, path_suffix: &str) -> String {
    url.trim_end_matches('/')
        .strip_suffix(path_suffix)
        .unwrap_or(url)
        .trim_end_matches('/')
        .to_string()
}

pub fn strip_code_fences(content: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string()
}

pub fn extract_json_payload(content: &str) -> String {
    let start = content.find('{').unwrap_or(0);
    let slice = &content[start..];
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in slice.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return slice[..=idx].to_string();
            }
        }
    }
    slice.to_string()
}

fn extract_anthropic_text(json: &serde_json::Value) -> Option<String> {
    json.get("content")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            items.iter().find_map(|item| {
                if item.get("type").and_then(|value| value.as_str()) == Some("text") {
                    item.get("text").and_then(|value| value.as_str())
                } else {
                    None
                }
            })
        })
        .map(str::to_string)
}

fn extract_openai_responses_text(json: &serde_json::Value) -> Option<String> {
    json.get("output")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            items.iter().find_map(|item| {
                item.get("content")
                    .and_then(|value| value.as_array())
                    .and_then(|content| {
                        content.iter().find_map(|part| {
                            if part.get("type").and_then(|value| value.as_str())
                                == Some("output_text")
                            {
                                part.get("text").and_then(|value| value.as_str())
                            } else {
                                None
                            }
                        })
                    })
            })
        })
        .map(str::to_string)
}

fn extract_anthropic_tool_input(json: &serde_json::Value, tool_name: &str) -> Option<String> {
    json.get("content")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            items.iter().find_map(|item| {
                let is_tool_use =
                    item.get("type").and_then(|value| value.as_str()) == Some("tool_use");
                let name_matches =
                    item.get("name").and_then(|value| value.as_str()) == Some(tool_name);
                if is_tool_use && name_matches {
                    item.get("input").map(json_value_to_payload_string)
                } else {
                    None
                }
            })
        })
}

fn extract_openai_tool_input(json: &serde_json::Value, tool_name: &str) -> Option<String> {
    json.get("output")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            items.iter().find_map(|item| {
                extract_openai_tool_input_from_item(item, tool_name).or_else(|| {
                    item.get("content")
                        .and_then(|value| value.as_array())
                        .and_then(|content| {
                            content.iter().find_map(|part| {
                                extract_openai_tool_input_from_item(part, tool_name)
                            })
                        })
                })
            })
        })
}

fn extract_openai_tool_input_from_item(
    item: &serde_json::Value,
    tool_name: &str,
) -> Option<String> {
    let item_type = item.get("type").and_then(|value| value.as_str());
    let type_matches = matches!(item_type, Some("function_call") | Some("tool_call"));
    let name_matches = item.get("name").and_then(|value| value.as_str()) == Some(tool_name);
    if type_matches && name_matches {
        item.get("arguments")
            .or_else(|| item.get("input"))
            .map(json_value_to_payload_string)
    } else {
        None
    }
}

fn json_value_to_payload_string(value: &serde_json::Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_anthropic_passage_tool_input() {
        let response = serde_json::json!({
            "content": [{
                "type": "tool_use",
                "name": "write_ai_passage",
                "input": {
                    "title": "海底计划",
                    "paragraphs": ["第一段 [[word:101]]。"]
                }
            }]
        });

        let payload =
            extract_anthropic_tool_input(&response, AI_PASSAGE_TOOL_NAME).expect("tool payload");
        let parsed: serde_json::Value = serde_json::from_str(&payload).expect("json payload");

        assert_eq!(parsed["title"], "海底计划");
        assert_eq!(parsed["paragraphs"][0], "第一段 [[word:101]]。");
    }

    #[test]
    fn extracts_openai_passage_tool_arguments() {
        let response = serde_json::json!({
            "output": [{
                "type": "function_call",
                "name": "write_ai_passage",
                "arguments": "{\"title\":\"风暴花园\",\"paragraphs\":[\"第二段 [[word:202]]。\"]}"
            }]
        });

        let payload =
            extract_openai_tool_input(&response, AI_PASSAGE_TOOL_NAME).expect("tool payload");
        let parsed: serde_json::Value = serde_json::from_str(&payload).expect("json payload");

        assert_eq!(parsed["title"], "风暴花园");
        assert_eq!(parsed["paragraphs"][0], "第二段 [[word:202]]。");
    }
}
