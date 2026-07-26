//! DeepSeek LLM Service for AI-powered assignment personalization.
//!
//! This module provides integration with DeepSeek's API (OpenAI-compatible)
//! to personalize assignments based on student context.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use thiserror::Error;

/// Errors that can occur during LLM operations
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("Missing API key: DEEPSEEK_API_KEY environment variable not set")]
    MissingApiKey,

    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Rate limited, retry after {retry_after_seconds} seconds")]
    RateLimited { retry_after_seconds: u64 },

    #[error("Invalid response structure: {0}")]
    InvalidResponse(String),
}

/// Configuration for the DeepSeek LLM client
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            model: "deepseek-chat".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

impl LlmConfig {
    /// Create config from environment variables
    pub fn from_env() -> Result<Self, LlmError> {
        let api_key = env::var("DEEPSEEK_API_KEY").map_err(|_| LlmError::MissingApiKey)?;

        Ok(Self {
            api_key,
            base_url: env::var("DEEPSEEK_BASE_URL")
                .unwrap_or_else(|_| "https://api.deepseek.com/v1".to_string()),
            model: env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-chat".to_string()),
            max_tokens: env::var("DEEPSEEK_MAX_TOKENS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4096),
            temperature: env::var("DEEPSEEK_TEMPERATURE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.7),
        })
    }
}

/// DeepSeek LLM client for assignment personalization
#[derive(Clone)]
pub struct DeepSeekClient {
    client: reqwest::Client,
    config: LlmConfig,
}

/// OpenAI-compatible chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI-compatible chat completion request
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

/// OpenAI-compatible chat completion response
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
    usage: Option<TokenUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// Personalized assignment output from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizedAssignment {
    pub personalized_title: String,
    pub personalized_body: String,
    pub scope: AssignmentScope,
    pub rubric: PersonalizedRubric,
    pub personalization_notes: String,
    pub estimated_difficulty: String,
}

/// Scope of the personalized assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentScope {
    #[serde(rename = "type")]
    pub assignment_type: String,
    pub estimated_hours: Option<f32>,
    pub page_count: Option<u32>,
    pub word_count: Option<u32>,
    pub deliverables: Vec<String>,
}

/// Personalized grading rubric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizedRubric {
    pub criteria: Vec<RubricCriterion>,
    pub total_points: u32,
}

/// Single criterion in the rubric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricCriterion {
    pub name: String,
    pub weight: u32,
    pub description: String,
    pub excellent: String,
    pub good: String,
    pub needs_improvement: String,
}

/// Student context for personalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentContext {
    pub student_id: String,
    pub student_name: String,
    pub talent_profile: Option<TalentProfile>,
    pub teacher_reports: Vec<TeacherReport>,
    pub previous_performance: PerformanceMetrics,
}

/// Student's talent profile
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TalentProfile {
    pub primary_talents: Vec<String>,
    pub learning_style: Option<String>,
    pub cognitive_strengths: Vec<String>,
    pub interests: Vec<String>,
    pub preferred_formats: Vec<String>,
}

/// Teacher's report/observation about a student
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeacherReport {
    pub teacher_name: String,
    pub subject: Option<String>,
    pub summary: String,
    pub date: String,
}

/// Student's historical performance metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    pub average_grade: Option<f32>,
    pub submission_rate: Option<f32>,
    pub on_time_rate: Option<f32>,
    pub strengths: Vec<String>,
    pub areas_for_improvement: Vec<String>,
}

/// Base assignment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseAssignment {
    pub title: String,
    pub body: String,
    pub subject: String,
    pub due_date: String,
    pub lecture_title: Option<String>,
    pub lecture_number: Option<i32>,
}

/// Course material context from RAG retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialContext {
    pub chunk_text: String,
    pub material_title: String,
    pub relevance_score: f32,
}

impl DeepSeekClient {
    /// Create a new DeepSeek client with config from environment
    pub fn new() -> Result<Self, LlmError> {
        let config = LlmConfig::from_env()?;
        Self::with_config(config)
    }

    /// Create a new DeepSeek client with custom config
    pub fn with_config(config: LlmConfig) -> Result<Self, LlmError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120)) // LLM calls can be slow
            .build()
            .map_err(LlmError::RequestFailed)?;

        Ok(Self { client, config })
    }

    /// Personalize an assignment for a specific student
    pub async fn personalize_assignment(
        &self,
        base_assignment: &BaseAssignment,
        student_context: &StudentContext,
    ) -> Result<PersonalizedAssignment, LlmError> {
        // Call the context-aware version with empty context for backward compatibility
        self.personalize_assignment_with_context(base_assignment, student_context, &[]).await
    }

    /// Personalize an assignment with course material context from RAG
    pub async fn personalize_assignment_with_context(
        &self,
        base_assignment: &BaseAssignment,
        student_context: &StudentContext,
        material_context: &[MaterialContext],
    ) -> Result<PersonalizedAssignment, LlmError> {
        let system_prompt = self.build_system_prompt_with_rag(!material_context.is_empty());
        let user_prompt = self.build_user_prompt_with_context(base_assignment, student_context, material_context)?;

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];

        let response = self.chat_completion(messages, true).await?;
        self.parse_personalized_assignment(&response)
    }

    /// Build the system prompt for assignment personalization
    fn build_system_prompt(&self) -> String {
        r#"You are an educational AI assistant specialized in personalizing assignments for students. Your role is to adapt assignments based on each student's unique profile:

1. **Cognitive Profile**: Learning style, processing speed, attention patterns
2. **Talent Areas**: Natural strengths and aptitudes identified by teachers
3. **Teacher Observations**: Historical feedback from educators about this student
4. **Previous Performance**: Past assignment completion patterns and quality

Your personalized assignments must be:
- **Achievable but challenging**: Match the student's current level while promoting growth
- **Aligned with learning style**: Visual learners get diagrams, kinesthetic get hands-on tasks, etc.
- **Appropriate in scope**: Adjust length, depth, and complexity based on the student
- **Tailored in format**: Writing, coding, projects, presentations based on their strengths

**CRITICAL RULES:**
1. NEVER give the same assignment to all students - each must be truly personalized
2. If a student struggles with writing, offer alternative formats (coding, presentations, diagrams)
3. If a student excels, increase depth and complexity, not just quantity
4. Always explain WHY you made specific personalization choices

**OUTPUT FORMAT: You must respond with valid JSON only, no markdown formatting.**"#.to_string()
    }

    /// Build the user prompt with assignment and student context
    fn build_user_prompt(
        &self,
        base_assignment: &BaseAssignment,
        student_context: &StudentContext,
    ) -> Result<String, LlmError> {
        let context = json!({
            "base_assignment": {
                "title": base_assignment.title,
                "body": base_assignment.body,
                "subject": base_assignment.subject,
                "due_date": base_assignment.due_date,
                "lecture_title": base_assignment.lecture_title,
                "lecture_number": base_assignment.lecture_number
            },
            "student_context": {
                "name": student_context.student_name,
                "talent_profile": student_context.talent_profile,
                "teacher_reports": student_context.teacher_reports,
                "previous_performance": student_context.previous_performance
            }
        });

        let prompt = format!(
            r#"Please personalize the following assignment for this specific student.

**CONTEXT:**
{}

**REQUIRED OUTPUT FORMAT:**
Respond with a JSON object containing:
- "personalized_title": A motivating title tailored to this student
- "personalized_body": The full assignment instructions, personalized for this student's abilities and style
- "scope": {{
    "type": "writing" | "coding" | "project" | "presentation" | "mixed",
    "estimated_hours": number or null,
    "page_count": number or null (if writing),
    "word_count": number or null (if writing),
    "deliverables": ["list", "of", "expected", "outputs"]
  }}
- "rubric": {{
    "criteria": [
      {{
        "name": "Criterion Name",
        "weight": percentage (0-100),
        "description": "What this criterion evaluates",
        "excellent": "Description of excellent performance",
        "good": "Description of good performance",
        "needs_improvement": "Description of needs improvement"
      }}
    ],
    "total_points": 100
  }}
- "personalization_notes": Explain what personalization choices you made and why
- "estimated_difficulty": "easy" | "medium" | "challenging" for THIS specific student

Remember: The goal is to help this student succeed by matching the assignment to their unique strengths and learning style."#,
            serde_json::to_string_pretty(&context).unwrap_or_default()
        );

        Ok(prompt)
    }

    /// Build system prompt with RAG context awareness
    fn build_system_prompt_with_rag(&self, has_material_context: bool) -> String {
        let base_prompt = r#"You are an educational AI assistant specialized in personalizing assignments for students. Your role is to adapt assignments based on each student's unique profile:

1. **Cognitive Profile**: Learning style, processing speed, attention patterns
2. **Talent Areas**: Natural strengths and aptitudes identified by teachers
3. **Teacher Observations**: Historical feedback from educators about this student
4. **Previous Performance**: Past assignment completion patterns and quality

Your personalized assignments must be:
- **Achievable but challenging**: Match the student's current level while promoting growth
- **Aligned with learning style**: Visual learners get diagrams, kinesthetic get hands-on tasks, etc.
- **Appropriate in scope**: Adjust length, depth, and complexity based on the student
- **Tailored in format**: Writing, coding, projects, presentations based on their strengths

**CRITICAL RULES:**
1. NEVER give the same assignment to all students - each must be truly personalized
2. If a student struggles with writing, offer alternative formats (coding, presentations, diagrams)
3. If a student excels, increase depth and complexity, not just quantity
4. Always explain WHY you made specific personalization choices"#;

        let rag_addition = if has_material_context {
            r#"

**IMPORTANT - COURSE MATERIALS CONTEXT:**
You have been provided with relevant excerpts from the course materials uploaded by the teacher.
USE this content to:
- Ensure the personalized assignment aligns with the actual curriculum content
- Reference specific concepts, examples, or topics from the materials
- Make the assignment more relevant to what students are actually learning
- Ground your personalization in the actual course content"#
        } else {
            ""
        };

        format!(
            "{}{}\n\n**OUTPUT FORMAT: You must respond with valid JSON only, no markdown formatting.**",
            base_prompt, rag_addition
        )
    }

    /// Build user prompt with course material context
    fn build_user_prompt_with_context(
        &self,
        base_assignment: &BaseAssignment,
        student_context: &StudentContext,
        material_context: &[MaterialContext],
    ) -> Result<String, LlmError> {
        let mut context = json!({
            "base_assignment": {
                "title": base_assignment.title,
                "body": base_assignment.body,
                "subject": base_assignment.subject,
                "due_date": base_assignment.due_date,
                "lecture_title": base_assignment.lecture_title,
                "lecture_number": base_assignment.lecture_number
            },
            "student_context": {
                "name": student_context.student_name,
                "talent_profile": student_context.talent_profile,
                "teacher_reports": student_context.teacher_reports,
                "previous_performance": student_context.previous_performance
            }
        });

        // Add course material context if available
        if !material_context.is_empty() {
            let materials: Vec<Value> = material_context.iter().map(|m| {
                json!({
                    "source": m.material_title,
                    "content": m.chunk_text,
                    "relevance": m.relevance_score
                })
            }).collect();
            context["course_materials"] = json!(materials);
        }

        let material_instruction = if !material_context.is_empty() {
            "\n\n**COURSE MATERIALS:** Use the provided course material excerpts to ensure the personalized assignment is grounded in actual curriculum content."
        } else {
            ""
        };

        let prompt = format!(
            r#"Please personalize the following assignment for this specific student.{}

**CONTEXT:**
{}

**REQUIRED OUTPUT FORMAT:**
Respond with a JSON object containing:
- "personalized_title": A motivating title tailored to this student
- "personalized_body": The full assignment instructions, personalized for this student's abilities and style
- "scope": {{
    "type": "writing" | "coding" | "project" | "presentation" | "mixed",
    "estimated_hours": number or null,
    "page_count": number or null (if writing),
    "word_count": number or null (if writing),
    "deliverables": ["list", "of", "expected", "outputs"]
  }}
- "rubric": {{
    "criteria": [
      {{
        "name": "Criterion Name",
        "weight": percentage (0-100),
        "description": "What this criterion evaluates",
        "excellent": "Description of excellent performance",
        "good": "Description of good performance",
        "needs_improvement": "Description of needs improvement"
      }}
    ],
    "total_points": 100
  }}
- "personalization_notes": Explain what personalization choices you made and why
- "estimated_difficulty": "easy" | "medium" | "challenging" for THIS specific student

Remember: The goal is to help this student succeed by matching the assignment to their unique strengths and learning style."#,
            material_instruction,
            serde_json::to_string_pretty(&context).unwrap_or_default()
        );

        Ok(prompt)
    }

    /// Make a chat completion request to DeepSeek API
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        json_mode: bool,
    ) -> Result<String, LlmError> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            response_format: if json_mode {
                Some(ResponseFormat {
                    format_type: "json_object".to_string(),
                })
            } else {
                None
            },
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            // Check for rate limiting
            if status.as_u16() == 429 {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);
                return Err(LlmError::RateLimited {
                    retry_after_seconds: retry_after,
                });
            }

            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let completion: ChatCompletionResponse = response.json().await.map_err(|e| {
            LlmError::ParseError(format!("Failed to parse completion response: {}", e))
        })?;

        let choice = completion.choices.into_iter().next().ok_or_else(|| {
            LlmError::InvalidResponse("No choices in completion response".to_string())
        })?;

        // Log token usage for cost tracking (in debug mode)
        if let Some(usage) = completion.usage {
            tracing::debug!(
                "LLM token usage - prompt: {}, completion: {}, total: {}",
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.total_tokens
            );
        }

        Ok(choice.message.content)
    }

    /// Parse the LLM response into a PersonalizedAssignment
    fn parse_personalized_assignment(&self, response: &str) -> Result<PersonalizedAssignment, LlmError> {
        // Try to extract JSON from the response (handle markdown code blocks)
        let json_str = if response.contains("```json") {
            response
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
                .trim()
        } else if response.contains("```") {
            response
                .split("```")
                .nth(1)
                .unwrap_or(response)
                .trim()
        } else {
            response.trim()
        };

        serde_json::from_str(json_str).map_err(|e| {
            LlmError::ParseError(format!(
                "Failed to parse personalized assignment: {}. Response was: {}",
                e,
                &response[..response.len().min(500)]
            ))
        })
    }

    /// Check if the client is properly configured
    pub fn is_configured(&self) -> bool {
        !self.config.api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.base_url, "https://api.deepseek.com/v1");
        assert_eq!(config.model, "deepseek-chat");
        assert_eq!(config.max_tokens, 4096);
        assert!((config.temperature - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_system_prompt_not_empty() {
        let config = LlmConfig {
            api_key: "test-key".to_string(),
            ..Default::default()
        };
        let client = DeepSeekClient::with_config(config).unwrap();
        let prompt = client.build_system_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("personalize"));
    }

    #[test]
    fn test_user_prompt_includes_context() {
        let config = LlmConfig {
            api_key: "test-key".to_string(),
            ..Default::default()
        };
        let client = DeepSeekClient::with_config(config).unwrap();
        
        let assignment = BaseAssignment {
            title: "Test Assignment".to_string(),
            body: "Write an essay".to_string(),
            subject: "English".to_string(),
            due_date: "2024-12-20".to_string(),
            lecture_title: None,
            lecture_number: None,
        };
        
        let context = StudentContext {
            student_id: "123".to_string(),
            student_name: "John Doe".to_string(),
            talent_profile: Some(TalentProfile {
                primary_talents: vec!["analytical".to_string()],
                learning_style: Some("visual".to_string()),
                ..Default::default()
            }),
            teacher_reports: vec![],
            previous_performance: PerformanceMetrics::default(),
        };
        
        let prompt = client.build_user_prompt(&assignment, &context).unwrap();
        assert!(prompt.contains("John Doe"));
        assert!(prompt.contains("Test Assignment"));
        assert!(prompt.contains("analytical"));
    }

    #[test]
    fn test_parse_valid_personalized_assignment() {
        let config = LlmConfig {
            api_key: "test-key".to_string(),
            ..Default::default()
        };
        let client = DeepSeekClient::with_config(config).unwrap();
        
        let response = r#"{
            "personalized_title": "Visual Essay on Climate Change",
            "personalized_body": "Create an infographic essay...",
            "scope": {
                "type": "mixed",
                "estimated_hours": 3.0,
                "page_count": 2,
                "word_count": 500,
                "deliverables": ["infographic", "short explanation"]
            },
            "rubric": {
                "criteria": [
                    {
                        "name": "Visual Design",
                        "weight": 40,
                        "description": "Quality of infographic",
                        "excellent": "Professional quality",
                        "good": "Clear and informative",
                        "needs_improvement": "Unclear or messy"
                    }
                ],
                "total_points": 100
            },
            "personalization_notes": "Student is a visual learner",
            "estimated_difficulty": "medium"
        }"#;
        
        let result = client.parse_personalized_assignment(response);
        assert!(result.is_ok());
        let assignment = result.unwrap();
        assert_eq!(assignment.personalized_title, "Visual Essay on Climate Change");
        assert_eq!(assignment.estimated_difficulty, "medium");
    }
}
