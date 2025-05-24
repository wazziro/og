use clap::Parser;
use std::path::PathBuf;
use std::fs;
use std::io::{self, Read, Write};
use chrono::{Local};

mod task_model;
mod markdown_parser;
mod markdown_formatter;
mod apply_logic;
mod calendar;

use task_model::Task;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)] // Removed trailing_var_arg = true
struct Cli {
    // Options first
    #[arg(long, short = 'f', global = true, help = "Input format (json or markdown)")]
    from: Option<String>,

    #[arg(long, short = 't', global = true, help = "Output format (json or markdown)")]
    to: Option<String>,

    #[arg(long, short = 'o', global = true, help = "Output file path. Writes to stdout if not specified.")]
    output: Option<String>,

    // Subcommand next
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(help = "Input file path (for conversion). Reads from stdin if not specified or if path is '-'.")] // Removed num_args(0..)
    input_file_conversion: Option<String>,
}

#[derive(Parser, Debug)]
enum Commands {
    #[command(about = "Format a Markdown task file")]
    Fmt {
        #[arg(help = "Input Markdown file path. Reads from stdin if not specified or if path is '-'.")]
        input_file: Option<String>,

        #[arg(long, short = 'i', help = "Modify the input file in-place. Conflicts with global --output (-o).", conflicts_with = "output")]
        in_place: bool,
    },
    #[command(about = "Apply Markdown changes to a JSON file")]
    Apply {
        #[arg(long, help = "Target JSON file path")] 
        target_json: PathBuf,
        #[arg(long, help = "Dry run without modifying the JSON file")]
        dry_run: bool,
    },
    #[command(about = "Display calendar events")]
    Cal {
        #[arg(long = "title", help = "Show only titles without time")]
        title: bool,
        #[arg(long = "next", short = 'n', help = "Show next business day events")]
        next: bool,
        #[arg(long = "all", short = 'a', help = "Show all events including all-day and hidden events")]
        all: bool,
    },
}

fn read_input(input_file_path: Option<&String>) -> Result<String, String> {
    match input_file_path {
        Some(path) if path != "-" => fs::read_to_string(path).map_err(|e| format!("Error reading input file '{}': {}", path, e)),
        _ => { 
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).map_err(|e| format!("Error reading from stdin: {}", e))?;
            Ok(buf)
        }
    }
}

fn write_output(output_file_path: Option<&String>, content: &str) -> Result<(), String> {
    match output_file_path {
        Some(path) => fs::write(path, content).map_err(|e| format!("Error writing to output file '{}': {}", path, e)),
        None => {
            print!("{}", content);
            io::stdout().flush().map_err(|e| format!("Error flushing stdout: {}", e))
        }
    }
}


#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let default_created_date = Local::now().date_naive();

    if let Some(command) = cli.command {
        match command {
            Commands::Fmt { input_file, in_place } => {
                if in_place && cli.output.is_some() {
                    return Err("Error: --in-place cannot be used with --output (-o).".to_string());
                }
                if in_place && (input_file.is_none() || input_file.as_deref() == Some("-")) {
                    return Err("Error: --in-place requires a named input file, not stdin.".to_string());
                }

                let input_content = read_input(input_file.as_ref())?;
                let tasks = markdown_parser::parse_markdown_document_to_tasks(&input_content, default_created_date)?;
                let formatted_markdown = markdown_formatter::format_tasks_to_markdown_document(&tasks);

                if in_place {
                    let path = input_file.unwrap();
                    fs::write(&path, formatted_markdown).map_err(|e| format!("Error writing back to file '{}': {}", path, e))?;
                    eprintln!("Formatted file in-place: {}", path);
                } else {
                    write_output(cli.output.as_ref(), &formatted_markdown)?;
                }
            },
            Commands::Apply { target_json, dry_run } => {
                let from_format = cli.from.as_ref().map(|s| s.to_lowercase()).unwrap_or_default();
                if from_format != "markdown" {
                    return Err("Error: --from must be 'markdown' for apply command.".to_string());
                }
                let input_content = read_input(None)?;
                let existing_json = fs::read_to_string(&target_json)
                    .map_err(|e| format!("Error reading JSON file '{}': {}", target_json.display(), e))?;
                let mut existing_tasks: Vec<Task> = Vec::new();
                for line in existing_json.lines() {
                    if line.trim().is_empty() { continue; }
                    let task: Task = serde_json::from_str(line)
                        .map_err(|e| format!("Error parsing JSON line '{}': {}", line, e))?;
                    existing_tasks.push(task);
                }
                let markdown_tasks = markdown_parser::parse_markdown_document_to_tasks(&input_content, default_created_date)?;
                let final_tasks = apply_logic::apply_changes(existing_tasks, markdown_tasks, default_created_date)?;
                if dry_run {
                    println!("Dry run summary:");
                    println!("Added tasks:");
                    for task in &final_tasks {
                        println!("{}", task.name);
                    }
                } else {
                    let json_out = final_tasks.iter()
                        .map(|t| serde_json::to_string(t).unwrap())
                        .collect::<Vec<_>>()
                        .join("\n");
                    fs::write(&target_json, json_out + "\n")
                        .map_err(|e| format!("Error writing JSON file '{}': {}", target_json.display(), e))?;
                    let markdown_out = markdown_formatter::format_tasks_to_markdown_document(&final_tasks);
                    print!("{}", markdown_out);
                }
            },
            Commands::Cal { title, next, all } => {
                let events_result = if next {
                    calendar::get_next_business_day_events(all).await
                } else {
                    calendar::get_today_events(all).await
                };
                
                match events_result {
                    Ok(events) => {
                        let output = calendar::format_events_output(&events, title);
                        print!("{}", output);
                    }
                    Err(e) => {
                        return Err(format!("Calendar error: {}", e));
                    }
                }
            }
        }
    } else {
        // Conversion mode (no subcommand)
        let from_format = cli.from.ok_or_else(|| "Error: --from <FORMAT> is required for conversion mode.".to_string())?.to_lowercase();
        let to_format = cli.to.ok_or_else(|| "Error: --to <FORMAT> is required for conversion mode.".to_string())?.to_lowercase();

        let input_content = read_input(cli.input_file_conversion.as_ref())?;

        match (from_format.as_str(), to_format.as_str()) {
            ("markdown", "json") => {
                let tasks = markdown_parser::parse_markdown_document_to_tasks(&input_content, default_created_date)?;
                let mut json_outputs: Vec<String> = Vec::new();
                for task in tasks {
                    json_outputs.push(serde_json::to_string(&task).map_err(|e| format!("Error serializing task to JSON: {}", e))?);
                }
                let output_string = json_outputs.join("\n");
                let final_output = if output_string.is_empty() { "".to_string() } else { output_string + "\n" };
                write_output(cli.output.as_ref(), &final_output)?;
            }
            ("json", "markdown") => {
                let mut tasks: Vec<Task> = Vec::new();
                for line in input_content.lines() {
                    if line.trim().is_empty() { continue; }
                    let task: Task = serde_json::from_str(line).map_err(|e| format!("Error deserializing task from JSON line '{}': {}", line, e))?;
                    tasks.push(task);
                }
                let markdown_output = markdown_formatter::format_tasks_to_markdown_document(&tasks);
                write_output(cli.output.as_ref(), &markdown_output)?;
            }
            _ => return Err(format!("Error: Unsupported conversion from '{}' to '{}'.", from_format, to_format)),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn create_test_task(id: i64, name: &str) -> Task {
        Task {
            id,
            name: name.to_string(),
            status: "open".to_string(),
            notes: None, // Changed from memo
            created: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), // Changed from created_at, removed time part as NaiveDate is date only
            updated: None, // Changed from updated_at
            due: None, // Changed from due_date
            completed: None, // Changed from completed_at
            tags: Some(Vec::new()), // Wrapped in Some()
            subtasks: Some(Vec::new()), // Changed from sub_tasks and wrapped in Some()
            // parent_id: None, // Removed, not in Task model
            priority: "N".to_string(), // Changed to String, provided a default
            // estimate_hours: None, // Removed
            // actual_hours: None, // Removed
            display_order: 1, // Added required field
            project: None, // Added optional field
            contexts: None, // Added optional field
            extra: None, // Added optional field
            repeat: None, // Added optional field
        }
    }

    #[test]
    fn test_markdown_to_json_conversion_logic() {
        let markdown_input = "- [ ] [[Task 1]]
- [x] [[Task 2]]";
        let default_date = Local::now().date_naive();
        let tasks = markdown_parser::parse_markdown_document_to_tasks(markdown_input, default_date).unwrap();
        
        let mut json_outputs: Vec<String> = Vec::new();
        for task in tasks {
            json_outputs.push(serde_json::to_string(&task).unwrap());
        }
        let result_json_string = json_outputs.join("\n");

        // Expected JSON structure might be complex to assert directly without knowing exact auto-generated IDs and timestamps.
        // For now, we'll check if the number of tasks is correct and if basic fields are present.
        assert_eq!(json_outputs.len(), 2);
        assert!(result_json_string.contains("\"name\":\"Task 1\""));
        assert!(result_json_string.contains("\"name\":\"Task 2\""));
        assert!(result_json_string.contains("\"status\":\"open\""));
        assert!(result_json_string.contains("\"status\":\"done\""));
    }

    #[test]
    fn test_json_to_markdown_conversion_logic() {
        let task1 = create_test_task(1, "Task 1 from JSON");
        let mut task2 = create_test_task(2, "Task 2 from JSON");
        // Ensure subtasks is Some before pushing
        if task2.subtasks.is_none() {
            task2.subtasks = Some(Vec::new());
        }
        task2.subtasks.as_mut().unwrap().push(create_test_task(3, "Subtask 2.1"));

        let json_input_task1 = serde_json::to_string(&task1).unwrap();
        let json_input_task2 = serde_json::to_string(&task2).unwrap();
        let json_input = format!("{}\n{}", json_input_task1, json_input_task2);

        let mut tasks: Vec<Task> = Vec::new();
        for line in json_input.lines() {
            if line.trim().is_empty() { continue; }
            let task: Task = serde_json::from_str(line).unwrap();
            tasks.push(task);
        }

        let markdown_output = markdown_formatter::format_tasks_to_markdown_document(&tasks);

        // Basic checks for presence of key elements
        assert!(markdown_output.contains("[[Task 1 from JSON]]"));
        assert!(markdown_output.contains("[[Task 2 from JSON]]"));
        assert!(markdown_output.contains("Subtask 2.1"));
    }

    #[test]
    fn test_fmt_logic_no_inplace() {
        let markdown_input = "- [ ] [[Task A]]
    - [x] [[Subtask B]]
- [ ] [[Task C]]";
        let default_date = Local::now().date_naive();
        
        let tasks = markdown_parser::parse_markdown_document_to_tasks(markdown_input, default_date).unwrap();
        let formatted_markdown = markdown_formatter::format_tasks_to_markdown_document(&tasks);

        // The formatter should produce a consistent output.
        // This test assumes the formatter correctly handles hierarchy and status.
        // The exact expected output depends on the formatter's rules (e.g., ID generation, date formatting).
        // For this test, we'll check for key elements.
        assert!(formatted_markdown.contains("[[Task A]]")); // Exact formatting might vary
        assert!(formatted_markdown.contains("[[Subtask B]]"));
        assert!(formatted_markdown.contains("[[Task C]]"));

        // A more robust test would compare against a precisely expected formatted string,
        // but that requires knowing the exact output of markdown_formatter including IDs.
        // For now, check if parsing and re-formatting doesn't lose tasks.
        let re_parsed_tasks = markdown_parser::parse_markdown_document_to_tasks(&formatted_markdown, default_date).unwrap();
        assert_eq!(re_parsed_tasks.len(), 2, "Formatting should preserve top-level task count.");
        assert_eq!(re_parsed_tasks[0].subtasks.as_ref().map_or(0, |s| s.len()), 1, "Formatting should preserve sub-task count for Task A."); // Changed from sub_tasks
    }

}
