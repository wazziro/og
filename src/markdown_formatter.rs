use crate::task_model::Task;

// B.3. 要素詳細 と B.4. 属性ごとの表示ルール に基づく
fn map_status_string_to_char(status_string: &str) -> char {
    // 仕様書とコードの差異: 
    // - 仕様書では大文字表記 (例: "NONE") を使用
    // - 実装では小文字表記 (例: "open") を使用
    // - 特に "open" と "none" は実装では同じ ' ' 文字にマップ
    match status_string.to_ascii_lowercase().as_str() {
        "none" | "open" => ' ',  // 仕様書では "NONE"
        "pending" => 'p',
        "doing" => '>',
        "waiting" => 'w',
        "done" => 'x',
        "cancelled" => 'c',
        _ => '?', // デフォルトまたはエラーケース
    }
}

// format_task_to_markdown_line を変更 (行頭マーカーとインデントは呼び出し元で付与)
fn format_task_core_content(task: &Task) -> String { // 新しい内部関数名
    let status_char = map_status_string_to_char(&task.status);
    let priority_str = &task.priority;
    let task_name_str = &task.name;

    let mut attributes: Vec<String> = Vec::new();

    // id (必須)
    attributes.push(format!("id:{}", task.id));

    // due (キー必須、値はOption<NaiveDate>)
    match &task.due {
        Some(due_date) => attributes.push(format!("due:{}", due_date.format("%Y-%m-%d"))),
        None => attributes.push("due:\"\"".to_string()),
    }

    // project (オプション)
    if let Some(project_name) = &task.project {
        attributes.push(format!("+{}", project_name));
    }

    // contexts (オプション、複数可)
    if let Some(contexts_vec) = &task.contexts {
        if !contexts_vec.is_empty() {
            let contexts_str = contexts_vec.iter().map(|c| format!("@{}", c)).collect::<Vec<String>>().join(" ");
            attributes.push(contexts_str);
        }
    }

    // tags (オプション、複数可)
    if let Some(tags_vec) = &task.tags {
        if !tags_vec.is_empty() {
            let tags_str = tags_vec.iter().map(|t| format!("#{}", t)).collect::<Vec<String>>().join(" ");
            attributes.push(tags_str);
        }
    }
    
    // created (必須, NaiveDate)
    attributes.push(format!("created:{}", task.created.format("%Y-%m-%d")));

    // updated (キー必須、値はOption<NaiveDate>)
    match &task.updated {
        Some(updated_date) => attributes.push(format!("updated:{}", updated_date.format("%Y-%m-%d"))),
        None => attributes.push("updated:\"\"".to_string()),
    }

    // completed (キー必須、値はOption<NaiveDate>)
    match &task.completed {
        Some(completed_date) => attributes.push(format!("completed:{}", completed_date.format("%Y-%m-%d"))),
        None => attributes.push("completed:\"\"".to_string()),
    }

    // notes (オプション)
    if let Some(note_str) = &task.notes {
        attributes.push(format!("note:\"{}\"", note_str.replace("\"", "\"\"")));
    }
    
    let attributes_combined_str = attributes.join(" ");

    // 行頭の "- " は除去。インデントは呼び出し側で。
    format!(
        "[{}] ({}) [[{}]] {}",
        status_char,
        priority_str,
        task_name_str,
        attributes_combined_str.trim_end()
    ).trim_end().to_string()
}

// 再帰的にタスクとサブタスクをフォーマットする内部ヘルパー
fn format_task_recursive_internal(task: &Task, indent_level: usize, lines: &mut Vec<String>) {
    let indent = "    ".repeat(indent_level); // 半角スペース4つで1レベル
    let task_core_line = format_task_core_content(task);
    lines.push(format!("{}- {}", indent, task_core_line));

    if let Some(subtasks) = &task.subtasks {
        for subtask in subtasks {
            format_task_recursive_internal(subtask, indent_level + 1, lines);
        }
    }
}

// 公開関数：Taskのスライスを受け取り、Markdownドキュメント文字列を生成
pub fn format_tasks_to_markdown_document(tasks: &[Task]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for task in tasks {
        // トップレベルタスクのインデントレベルは0
        format_task_recursive_internal(task, 0, &mut lines);
    }
    lines.join("\n")
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_model::Task;
    use chrono::NaiveDate; // Add this for NaiveDate literals

    #[test]
    fn test_format_single_task_no_subtasks() {
        let task = Task {
            name: "Simple Task".to_string(),
            status: "PENDING".to_string(),
            priority: "A".to_string(),
            id: 1,
            created: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            display_order: 1, 
            due: Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
            updated: None,
            completed: None,
            project: Some("MyProject".to_string()),
            contexts: Some(vec!["work".to_string(), "home".to_string()]),
            notes: Some("This is a note.".to_string()),
            tags: Some(vec!["important".to_string()]),
            subtasks: None,
            extra: None,
            repeat: None,
        };
        let expected_md = "- [p] (A) [[Simple Task]] id:1 due:2024-12-31 +MyProject @work @home #important created:2024-01-01 updated:\"\" completed:\"\" note:\"This is a note.\"";
        assert_eq!(format_tasks_to_markdown_document(&[task]), expected_md);
    }

    #[test]
    fn test_format_minimal_task_document() {
        let task = Task {
            name: "Minimal Task".to_string(),
            status: "NONE".to_string(),
            priority: "N".to_string(),
            id: 2,
            created: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            display_order: 2,
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
        };
        let expected_md = "- [ ] (N) [[Minimal Task]] id:2 due:\"\" created:2024-01-02 updated:\"\" completed:\"\"";
        assert_eq!(format_tasks_to_markdown_document(&[task]), expected_md);
    }

    #[test]
    fn test_format_task_with_empty_note_document() {
        let task = Task {
            name: "Empty Note Task".to_string(),
            status: "DONE".to_string(),
            priority: "C".to_string(),
            id: 3,
            created: NaiveDate::from_ymd_opt(2024, 3, 3).unwrap(),
            display_order: 3,
            due: Some(NaiveDate::from_ymd_opt(2024, 3, 10).unwrap()),
            updated: Some(NaiveDate::from_ymd_opt(2024, 3, 4).unwrap()),
            completed: Some(NaiveDate::from_ymd_opt(2024, 3, 5).unwrap()),
            project: None,
            contexts: None,
            notes: Some("".to_string()), 
            tags: None,
            subtasks: None,
            extra: None,
            repeat: None,
        };
        let expected_md = "- [x] (C) [[Empty Note Task]] id:3 due:2024-03-10 created:2024-03-03 updated:2024-03-04 completed:2024-03-05 note:\"\"";
        assert_eq!(format_tasks_to_markdown_document(&[task]), expected_md);
    }
    
    #[test]
    fn test_format_task_with_quotes_in_note_document() {
        let task = Task {
            name: "Note with quotes".to_string(),
            status: "PENDING".to_string(),
            priority: "B".to_string(),
            id: 4,
            created: NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(),
            display_order: 4,
            due: None,
            updated: None,
            completed: None,
            project: None,
            contexts: None,
            notes: Some("This is a \"quoted\" note.".to_string()),
            tags: None,
            subtasks: None,
            extra: None,
            repeat: None,
        };
        let expected_md = "- [p] (B) [[Note with quotes]] id:4 due:\"\" created:2024-07-01 updated:\"\" completed:\"\" note:\"This is a \"\"quoted\"\" note.\"";
        assert_eq!(format_tasks_to_markdown_document(&[task]), expected_md);
    }

    #[test]
    fn test_format_multiple_tasks_no_subtasks() {
        let task1_created = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let task1 = Task { id: 1, name: "Task 1".to_string(), status: "NONE".to_string(), priority: "N".to_string(), created: task1_created, display_order: 1, due: None, updated: None, completed: None, project: None, contexts: None, notes: None, tags: None, subtasks: None, extra: None, repeat: None };
        
        let task2_created = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
        let task2_due = Some(NaiveDate::from_ymd_opt(2024, 1, 10).unwrap());
        let task2_completed = Some(NaiveDate::from_ymd_opt(2024, 1, 3).unwrap());
        let task2 = Task { id: 2, name: "Task 2".to_string(), status: "DONE".to_string(), priority: "A".to_string(), created: task2_created, display_order: 2, due: task2_due, updated: None, completed: task2_completed, project: None, contexts: None, notes: None, tags: None, subtasks: None, extra: None, repeat: None };
        
        let expected_md = "\
- [ ] (N) [[Task 1]] id:1 due:\"\" created:2024-01-01 updated:\"\" completed:\"\"
- [x] (A) [[Task 2]] id:2 due:2024-01-10 created:2024-01-02 updated:\"\" completed:2024-01-03";
        assert_eq!(format_tasks_to_markdown_document(&[task1, task2]), expected_md);
    }

    #[test]
    fn test_format_task_with_simple_subtask() {
        let child_created = NaiveDate::from_ymd_opt(2024, 7, 15).unwrap();
        let child_task = Task {
            name: "Child Task".to_string(), status: "PENDING".to_string(), priority: "N".to_string(), id: 11, created: child_created, display_order: 2,
            due: None, updated: None, completed: None, project: None, contexts: None, notes: None, tags: None, subtasks: None, extra: None, repeat: None,
        };
        
        let parent_created = NaiveDate::from_ymd_opt(2024, 7, 15).unwrap();
        let parent_task = Task {
            name: "Parent Task".to_string(), status: "NONE".to_string(), priority: "A".to_string(), id: 10, created: parent_created, display_order: 1,
            due: None, updated: None, completed: None, project: None, contexts: None, notes: None, tags: None, subtasks: Some(vec![child_task]), extra: None, repeat: None,
        };
        let expected_md = "\
- [ ] (A) [[Parent Task]] id:10 due:\"\" created:2024-07-15 updated:\"\" completed:\"\"
    - [p] (N) [[Child Task]] id:11 due:\"\" created:2024-07-15 updated:\"\" completed:\"\"";
        assert_eq!(format_tasks_to_markdown_document(&[parent_task]), expected_md);
    }

    #[test]
    fn test_format_task_with_multiple_subtasks_and_levels() {
        let test_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // Common date for simplicity

        let gc1 = Task { name: "Grandchild 1.1.1".to_string(), id:3, status:"NONE".to_string(), priority:"N".to_string(), created:test_date, display_order:3, due:None,updated:None,completed:None,project:None,contexts:None,notes:None,tags:None,subtasks:None,extra:None,repeat:None };
        let c1 = Task { name: "Child 1.1".to_string(), id:2, status:"NONE".to_string(), priority:"N".to_string(), created:test_date, display_order:2, due:None,updated:None,completed:None,project:None,contexts:None,notes:None,tags:None,subtasks:Some(vec![gc1]),extra:None,repeat:None };
        let c2 = Task { name: "Child 1.2".to_string(), id:4, status:"NONE".to_string(), priority:"N".to_string(), created:test_date, display_order:4, due:None,updated:None,completed:None,project:None,contexts:None,notes:None,tags:None,subtasks:None,extra:None,repeat:None };
        let p1 = Task { name: "Parent 1".to_string(), id:1, status:"NONE".to_string(), priority:"N".to_string(), created:test_date, display_order:1, due:None,updated:None,completed:None,project:None,contexts:None,notes:None,tags:None,subtasks:Some(vec![c1, c2]),extra:None,repeat:None };

        let gc2_1_1 = Task { name: "GrandGrandchild 2.1.1".to_string(), id:7, status:"NONE".to_string(), priority:"N".to_string(), created:test_date, display_order:7, due:None,updated:None,completed:None,project:None,contexts:None,notes:None,tags:None,subtasks:None,extra:None,repeat:None };
        let c3 = Task { name: "Child 2.1".to_string(), id:6, status:"NONE".to_string(), priority:"N".to_string(), created:test_date, display_order:6, due:None,updated:None,completed:None,project:None,contexts:None,notes:None,tags:None,subtasks:Some(vec![gc2_1_1]),extra:None,repeat:None };
        let p2 = Task { name: "Parent 2".to_string(), id:5, status:"NONE".to_string(), priority:"N".to_string(), created:test_date, display_order:5, due:None,updated:None,completed:None,project:None,contexts:None,notes:None,tags:None,subtasks:Some(vec![c3]),extra:None,repeat:None };

        let expected_md = "\
- [ ] (N) [[Parent 1]] id:1 due:\"\" created:2024-01-01 updated:\"\" completed:\"\"
    - [ ] (N) [[Child 1.1]] id:2 due:\"\" created:2024-01-01 updated:\"\" completed:\"\"
        - [ ] (N) [[Grandchild 1.1.1]] id:3 due:\"\" created:2024-01-01 updated:\"\" completed:\"\"
    - [ ] (N) [[Child 1.2]] id:4 due:\"\" created:2024-01-01 updated:\"\" completed:\"\"
- [ ] (N) [[Parent 2]] id:5 due:\"\" created:2024-01-01 updated:\"\" completed:\"\"
    - [ ] (N) [[Child 2.1]] id:6 due:\"\" created:2024-01-01 updated:\"\" completed:\"\"
        - [ ] (N) [[GrandGrandchild 2.1.1]] id:7 due:\"\" created:2024-01-01 updated:\"\" completed:\"\"";
        assert_eq!(format_tasks_to_markdown_document(&[p1, p2]), expected_md);
    }
}
