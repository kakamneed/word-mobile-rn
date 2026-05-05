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

    pub fn run_text_json(
        &self,
        system_message: &str,
        user_message: &str,
    ) -> Result<String, String> {
        self.call_primary_text(system_message, user_message)
            .or_else(|primary_error| {
                self.call_backup_text(system_message, user_message, &primary_error)
            })
    }

    pub fn run_image_json(
        &self,
        system_message: &str,
        user_message: &str,
        image: ImageInput<'_>,
    ) -> Result<String, String> {
        self.call_primary_image(system_message, user_message, &image)
            .or_else(|primary_error| {
                self.call_backup_image(system_message, user_message, &image, &primary_error)
            })
    }

    fn primary_url(&self) -> String {
        normalize_provider_url(&self.config.primary.base_url, ANTHROPIC_MESSAGES_PATH)
    }

    fn backup_url(&self) -> String {
        normalize_provider_url(&self.config.backup.base_url, OPENAI_RESPONSES_PATH)
    }

    fn call_primary_text(
        &self,
        system_message: &str,
        user_message: &str,
    ) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .map_err(|e| format!("Primary AI client failed: {e}"))?;
        let response = client
            .post(self.primary_url())
            .header("x-api-key", &self.config.primary.auth_token)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": &self.config.primary.model,
                "max_tokens": 2048,
                "system": system_message,
                "messages": [{ "role": "user", "content": user_message }],
            }))
            .send()
            .map_err(|e| format!("Primary AI request failed: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("Primary AI request failed: {}", response.status()));
        }
        let json: serde_json::Value = response
            .json()
            .map_err(|e| format!("Primary AI response decode failed: {e}"))?;
        extract_anthropic_text(&json)
            .ok_or_else(|| "Primary AI response missing text content".to_string())
    }

    fn call_primary_image(
        &self,
        system_message: &str,
        user_message: &str,
        image: &ImageInput<'_>,
    ) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .map_err(|e| format!("Primary AI image client failed: {e}"))?;
        let response = client
            .post(self.primary_url())
            .header("x-api-key", &self.config.primary.auth_token)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": &self.config.primary.model,
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
            }))
            .send()
            .map_err(|e| format!("Primary AI image request failed: {e}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "Primary AI image request failed: {}",
                response.status()
            ));
        }
        let json: serde_json::Value = response
            .json()
            .map_err(|e| format!("Primary AI image response decode failed: {e}"))?;
        extract_anthropic_text(&json)
            .ok_or_else(|| "Primary AI image response missing text content".to_string())
    }

    fn call_backup_text(
        &self,
        system_message: &str,
        user_message: &str,
        upstream_error: &str,
    ) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .map_err(|e| format!("Backup AI client failed after {upstream_error}: {e}"))?;
        let response = client
            .post(self.backup_url())
            .header(
                "Authorization",
                format!("Bearer {}", self.config.backup.auth_token),
            )
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": &self.config.backup.model,
                "instructions": system_message,
                "reasoning": { "effort": "low" },
                "input": user_message,
                "store": false,
                "text": {
                    "format": { "type": "json_object" },
                    "verbosity": "medium",
                }
            }))
            .send()
            .map_err(|e| format!("Backup AI request failed after {upstream_error}: {e}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "Backup AI request failed after {upstream_error}: {}",
                response.status()
            ));
        }
        let json: serde_json::Value = response
            .json()
            .map_err(|e| format!("Backup AI response decode failed after {upstream_error}: {e}"))?;
        extract_openai_responses_text(&json)
            .ok_or_else(|| format!("Backup AI response missing output_text after {upstream_error}"))
    }

    fn call_backup_image(
        &self,
        system_message: &str,
        user_message: &str,
        image: &ImageInput<'_>,
        upstream_error: &str,
    ) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .map_err(|e| format!("Backup AI image client failed after {upstream_error}: {e}"))?;
        let response = client
            .post(self.backup_url())
            .header(
                "Authorization",
                format!("Bearer {}", self.config.backup.auth_token),
            )
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
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
                "store": false,
                "text": {
                    "format": { "type": "json_object" },
                    "verbosity": "medium",
                }
            }))
            .send()
            .map_err(|e| format!("Backup AI image request failed after {upstream_error}: {e}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "Backup AI image request failed after {upstream_error}: {}",
                response.status()
            ));
        }
        let json: serde_json::Value = response.json().map_err(|e| {
            format!("Backup AI image response decode failed after {upstream_error}: {e}")
        })?;
        extract_openai_responses_text(&json).ok_or_else(|| {
            format!("Backup AI image response missing output_text after {upstream_error}")
        })
    }
}

pub const ANTHROPIC_MESSAGES_PATH: &str = "/v1/messages";
pub const OPENAI_RESPONSES_PATH: &str = "/v1/responses";

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
