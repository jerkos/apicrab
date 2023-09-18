use crate::commands::run::_printer::Printer;
use crate::db::db_handler::DBHandler;
use crate::db::dto::{Action, Context};
use crate::http::FetchResult;
use crate::json_path;
use colored::Colorize;
use crossterm::style::Stylize;
use std::collections::HashMap;

pub struct HttpResult<'a> {
    pub db_handler: &'a DBHandler,
    pub fetch_result: &'a anyhow::Result<FetchResult>,
    pub printer: &'a mut Printer,
}

impl<'a> HttpResult<'a> {
    pub fn new(
        db_handler: &'a DBHandler,
        fetch_result: &'a anyhow::Result<FetchResult>,
        printer: &'a mut Printer,
    ) -> Self {
        Self {
            db_handler,
            fetch_result,
            printer,
        }
    }

    fn extract_pattern(
        &mut self,
        (pattern_to_extract, value_name): (&str, Option<&str>),
        response: &str,
    ) -> Option<String> {
        //anyhow::Result<String> {
        let extracted = json_path::json_path(response, pattern_to_extract);

        let extracted_as_string = extracted
            .map(|value| match serde_json::to_string_pretty(&value) {
                Ok(v) => {
                    if v.starts_with('\"') {
                        v[1..v.len() - 1].to_owned()
                    } else {
                        v
                    }
                }
                Err(_) => "".to_owned(),
            })
            .unwrap();

        if extracted_as_string.is_empty() {
            return None;
        }

        self.printer.p_info(|| {
            println!(
                "Extraction of {}: {} {}",
                pattern_to_extract.bright_green(),
                extracted_as_string.bright_magenta(),
                value_name
                    .map(|v| format!("saved as {}", v.bright_yellow()))
                    .unwrap_or("".to_string())
            )
        });

        Some(extracted_as_string)
    }

    fn print_response(&mut self, response: &str) -> anyhow::Result<()> {
        // grep is superior to no_print option
        self.printer.p_response(|| println!("{}", response));
        // print response as info if needed
        self.printer.p_info(|| {
            println!("Received response: ");
            let response_as_value = serde_json::from_str::<serde_json::Value>(response)
                .unwrap_or(serde_json::Value::Null);
            println!(
                "{}",
                serde_json::to_string_pretty(&response_as_value)
                    .unwrap_or("".to_string())
                    .split('\n')
                    .take(10)
                    .collect::<Vec<&str>>()
                    .join("\n")
                    .red()
            );
            println!("...");
        });

        // save response to clipboard if necessary
        self.printer.maybe_to_clip(response);
        Ok(())
    }

    pub async fn handle_result(
        &mut self,
        action: &mut Action,
        body: &Option<String>,
        extract_pattern: &Option<HashMap<String, Option<String>>>,
        ctx: &mut HashMap<String, String>,
    ) -> anyhow::Result<()> {
        match self.fetch_result {
            Ok(FetchResult {
                response, status, ..
            }) => {
                let status_code = format!("Status code: {}", status);
                if status >= &400 {
                    self.printer
                        .p_info(|| println!("{}", status_code.bold().red()));
                    self.printer.p_response(|| println!("{}", response));
                    return Ok(());
                }

                // Successful request
                self.printer
                    .p_info(|| println!("{}", status_code.bold().green()));

                action.response_example = Some(response.clone());
                action.body_example = body.clone();
                self.db_handler.upsert_action(action, self.printer).await?;

                match extract_pattern {
                    Some(pattern) => {
                        // qualify extract
                        let concat_pattern = pattern
                            .iter()
                            .filter_map(|(pattern, value_name)| {
                                let extracted_pattern =
                                    self.extract_pattern((pattern, None), response);
                                if let (Some(value_name), Some(extracted_pattern)) =
                                    (value_name, extracted_pattern.as_ref())
                                {
                                    ctx.insert(value_name.to_string(), extracted_pattern.clone());
                                }

                                extracted_pattern
                            })
                            .collect::<Vec<String>>()
                            .join("\n");

                        // to clip if necessary and print response if grepped
                        self.printer.maybe_to_clip(&concat_pattern);
                        self.printer.p_response(|| println!("{}", concat_pattern));

                        self.db_handler
                            .insert_conf(&Context {
                                value: serde_json::to_string(&ctx)
                                    .expect("Error serializing context"),
                            })
                            .await?;
                    }
                    None => self.print_response(response)?,
                }
            }
            Err(e) => println!("Error: {}", e),
        };

        Ok(())
    }
}