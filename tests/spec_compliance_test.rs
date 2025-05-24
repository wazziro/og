#[cfg(test)]
mod spec_compliance_tests {
    use chrono::{NaiveDate, Local, Datelike};
    use std::collections::HashMap;

    // Import needed modules
    use og::task_model::Task;
    use og::markdown_parser;
    use og::markdown_formatter;
    use og::apply_logic;

    // Helper function to create a standard test task
    fn create_test_task(id: i64, name: &str) -> Task {
        Task {
            id,
            name: name.to_string(),
            status: "open".to_string(), // Using lowercase as in implementation
            priority: "N".to_string(),
            created: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            display_order: id,
            due: None,
            updated: None,
            completed: None,
            project: None,
            contexts: None,
            notes: None,
            tags: None,
            subtasks: None,
            extra: None,
            repeat: None,
        }
    }

    #[test]
    fn test_date_format_parsing_compliance() {
        // Test according to spec B.5:
        // Formats supported for date parsing:
        // - YYYY-MM-DD (ISO format)
        // - YYYY/MM/DD (Slash format)
        // - MM/DD (Current year, two-digit month/day)
        // - M/D (Current year, single-digit month/day)

        let current_year = Local::now().year();
        let default_date = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();

        // YYYY-MM-DD format
        let md1 = "- [ ] [[Test Task 1]] created:2023-05-15 due:2024-06-30";
        let task1 = markdown_parser::parse_markdown_document_to_tasks(md1, default_date).unwrap()[0].clone();
        assert_eq!(task1.created, NaiveDate::from_ymd_opt(2023, 5, 15).unwrap());
        assert_eq!(task1.due, Some(NaiveDate::from_ymd_opt(2024, 6, 30).unwrap()));

        // YYYY/MM/DD format
        let md2 = "- [ ] [[Test Task 2]] created:2023/05/15 due:2024/06/30";
        let task2 = markdown_parser::parse_markdown_document_to_tasks(md2, default_date).unwrap()[0].clone();
        assert_eq!(task2.created, NaiveDate::from_ymd_opt(2023, 5, 15).unwrap());
        assert_eq!(task2.due, Some(NaiveDate::from_ymd_opt(2024, 6, 30).unwrap()));

        // MM/DD format (current year)
        let md3 = "- [ ] [[Test Task 3]] created:05/15 due:06/30";
        let task3 = markdown_parser::parse_markdown_document_to_tasks(md3, default_date).unwrap()[0].clone();
        assert_eq!(task3.created, NaiveDate::from_ymd_opt(current_year, 5, 15).unwrap());
        assert_eq!(task3.due, Some(NaiveDate::from_ymd_opt(current_year, 6, 30).unwrap()));

        // M/D format (current year, single-digit) - ensuring this works as per spec
        let md4 = "- [ ] [[Test Task 4]] created:5/5 due:6/9";
        let task4 = markdown_parser::parse_markdown_document_to_tasks(md4, default_date).unwrap()[0].clone();
        assert_eq!(task4.created, NaiveDate::from_ymd_opt(current_year, 5, 5).unwrap());
        assert_eq!(task4.due, Some(NaiveDate::from_ymd_opt(current_year, 6, 9).unwrap()));
    }

    #[test]
    fn test_attribute_deletion_handling() {
        // Test according to spec E.7:
        // - Optional keys are removed when deleted from MD
        // - Required keys with nullable values keep the key but set value to null

        let today = Local::now().date_naive();
        
        // Create existing task with all fields populated
        let mut existing_task = create_test_task(1, "Original Task");
        existing_task.project = Some("ProjectX".to_string());
        existing_task.due = Some(NaiveDate::from_ymd_opt(2023, 12, 31).unwrap());
        existing_task.contexts = Some(vec!["work".to_string()]);
        
        let mut extra_data = HashMap::new();
        extra_data.insert("custom_key".to_string(), serde_json::json!("custom_value"));
        existing_task.extra = Some(extra_data);
        
        let existing_tasks = vec![existing_task];
        
        // Create the markdown version with some fields deleted
        // - 'project' (optional key) is removed - should be removed from JSON
        // - 'due' (required key with nullable value) is removed - should be set to null in JSON
        // - 'contexts' (optional key) is removed - should be removed from JSON
        // - 'extra' is preserved as it's not directly editable in Markdown
        
        let markdown_task = "- [ ] [[Original Task]] id:1"; // Minimal representation with just ID and name
        let markdown_tasks = markdown_parser::parse_markdown_document_to_tasks(markdown_task, today).unwrap();
        
        // Apply the changes
        let result = apply_logic::apply_changes(existing_tasks, markdown_tasks, today).unwrap();
        assert_eq!(result.len(), 1);
        
        let updated_task = &result[0];
        
        // Optional keys should be removed
        assert!(updated_task.project.is_none(), "Optional key 'project' should be removed");
        assert!(updated_task.contexts.is_none(), "Optional key 'contexts' should be removed");
        
        // Required keys with nullable values should be set to null (None)
        assert!(updated_task.due.is_none(), "Required key 'due' should be set to null");
        
        // JSON-specific fields should be preserved
        assert!(updated_task.extra.is_some(), "JSON-specific 'extra' field should be preserved");
        assert_eq!(
            updated_task.extra.as_ref().unwrap().get("custom_key").unwrap(),
            &serde_json::json!("custom_value")
        );
        
        // Updated date should be set
        assert_eq!(updated_task.updated, Some(today));
    }

    #[test]
    fn test_task_status_mapping() {
        // Test the status mapping compliance between spec and implementation
        // Note on status mapping differences:
        // - Spec uses uppercase status values (e.g., "NONE", "PENDING")
        // - Implementation uses lowercase status values (e.g., "open", "pending")
        // - "open" in implementation corresponds to "NONE" in spec
        
        let default_date = Local::now().date_naive();
        
        // Test lowercase status in markdown
        let md1 = "- [x] [[Done Task]] id:1";
        let task1 = markdown_parser::parse_markdown_document_to_tasks(md1, default_date).unwrap()[0].clone();
        assert_eq!(task1.status, "done");  // Implementation uses lowercase "done" (spec: "DONE")
        
        let md2 = "- [p] [[Pending Task]] id:2";
        let task2 = markdown_parser::parse_markdown_document_to_tasks(md2, default_date).unwrap()[0].clone();
        assert_eq!(task2.status, "pending");  // Implementation uses lowercase "pending" (spec: "PENDING")
        
        let md3 = "- [ ] [[Open Task]] id:3";
        let task3 = markdown_parser::parse_markdown_document_to_tasks(md3, default_date).unwrap()[0].clone();
        assert_eq!(task3.status, "open"); // Implementation uses lowercase "open" (spec: "NONE")
        
        // Verify that roundtrip preserves status
        let tasks = vec![task1, task2, task3];
        let markdown = markdown_formatter::format_tasks_to_markdown_document(&tasks);
        
        // Round-trip back to Tasks
        let round_trip_tasks = markdown_parser::parse_markdown_document_to_tasks(&markdown, default_date).unwrap();
        
        assert_eq!(round_trip_tasks[0].status, "done");
        assert_eq!(round_trip_tasks[1].status, "pending");
        assert_eq!(round_trip_tasks[2].status, "open");
    }
}