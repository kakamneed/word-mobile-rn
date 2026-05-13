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
