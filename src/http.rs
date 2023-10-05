use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::time::{Duration, Instant};

use reqwest::multipart::{Form, Part};
use reqwest::Method;

use crate::db::db_handler::DBHandler;
use crate::db::dto::History;

#[derive(Debug, Clone)]
pub struct FetchResult {
    pub response: String,
    pub status: u16,
    pub duration: Duration,
}

static URL_ENCODED: &str = "application/x-www-form-urlencoded";
static FORM_DATA: &str = "multipart/form-data";
static APPLICATION_JSON: &str = "application/json";

pub struct Api<'a> {
    pub(crate) client: reqwest::Client,
    pub db_handler: &'a DBHandler,
}

impl<'a> Api<'a> {
    pub fn new(db_handler: &'a DBHandler) -> Self {
        Self {
            client: reqwest::Client::new(),
            db_handler,
        }
    }

    /// insert history line
    pub async fn save_history_line(
        &self,
        action_name: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: &Option<Cow<'a, str>>,
        fetch_result: &FetchResult,
    ) -> anyhow::Result<()> {
        // insert history line !
        self.db_handler
            .insert_history(&History {
                id: None,
                action_name: action_name.to_string(),
                url: url.to_string(),
                body: body.as_ref().map(|s| s.to_string()),
                headers: Some(serde_json::to_string(headers)?),
                response: Some(fetch_result.response.clone()),
                status_code: fetch_result.status,
                duration: fetch_result.duration.as_secs_f32(),
                timestamp: None,
            })
            .await?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn fetch(
        &self,
        action_name: &str,
        url: &str,
        verb: &str,
        headers: &HashMap<String, String>,
        query_params: &Option<HashMap<String, String>>,
        body: &Option<Cow<'a, str>>,
    ) -> anyhow::Result<FetchResult> {
        // building request
        let mut builder = match verb {
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "GET" => self.client.get(url),
            "DELETE" => self.client.delete(url),
            "OPTIONS" => self.client.request(Method::OPTIONS, url),
            _ => panic!("Unsupported verb: {}", verb),
        };

        // query params
        if query_params.is_some() {
            builder = builder.query(query_params.as_ref().unwrap());
        }

        // headers are automatically set !

        // body
        let content_type = headers
            .get(reqwest::header::CONTENT_TYPE.as_str())
            .map(String::as_str)
            .unwrap_or(APPLICATION_JSON);
        let is_url_encoded = content_type == URL_ENCODED;
        let is_form_data = content_type == FORM_DATA;

        builder = match (is_url_encoded, is_form_data) {
            (true, true) => panic!("Cannot have both url encoded and form data"),
            (false, false) => {
                if body.is_some() {
                    builder = builder.body(body.as_ref().unwrap().to_string());
                }
                builder
            }
            (true, false) => builder.form(&serde_json::from_str::<HashMap<String, String>>(
                body.as_ref().unwrap(),
            )?),
            (false, true) => {
                let mut form = Form::new();
                let body = body.as_ref().unwrap();
                for (part_name, v) in serde_json::from_str::<HashMap<String, String>>(body)? {
                    // handle file upload
                    if v.starts_with('@') {
                        let file_path = v.trim_start_matches('@');
                        form = form.part(
                            part_name,
                            Part::bytes(fs::read(file_path)?).file_name(file_path.to_string()),
                        );
                        continue;
                    }
                    form = form.text(part_name, v);
                }
                builder.multipart(form)
            }
        };

        // launching request
        let start = Instant::now();
        let response = builder.send().await?;
        let duration = start.elapsed();

        // getting status and response
        let status = response.status();
        let text: String = response.text().await?;

        let fetch_result = FetchResult {
            response: text,
            status: status.as_u16(),
            duration,
        };
        // insert history line
        self.save_history_line(action_name, url, headers, body, &fetch_result)
            .await?;

        // return results
        Ok(fetch_result)
    }
}
