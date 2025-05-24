use crate::task_model::Task;
use chrono::{Local, NaiveDate};
use std::collections::{HashMap, HashSet};

// D.4. 差分マージロジックの基本方針
// 1. タスクの同定: id フィールドを使用。
// 2. フィールドの更新: Markdownで編集可能なフィールドはJSONに値を反映。updated 日はツール処理日で自動更新。
// 3. 新規タスクの追加: Markdownでの新規追加は、JSON仕様に従い id, created, display_order を採番・設定してJSONに追加。
// 4. タスクの削除: Markdownからタスク行が削除されたらJSONからも対応タスクを削除（確認なし）。
// 5. タスクの順序変更: Markdownの行順変更を検出し、JSON側の全タスクの display_order を再採番して反映。
// 6. JSON固有情報の保護: extra フィールドなど、Markdownで直接編集されないJSON側の情報は保持。
// 7. 属性削除の扱い:
//    - オプションキー属性 (project等) がMDから削除されたら、JSONからもキーごと削除。
//      (実装: パーサーは属性がない場合にNoneを設定し、apply_changesがこれを反映してキーごと削除)
//    - キー必須（値null可）属性 (due等) がMDからまるごと削除されたら、JSONではキーを残し値をnullに。
//      (実装: パーサーは属性がない場合にNoneを設定し、apply_changesがこれを反映してnull値を設定)

pub fn apply_changes(
    existing_tasks_vec: Vec<Task>,
    markdown_tasks_vec: Vec<Task>,
    _default_created_date: NaiveDate, // May be needed for new tasks if not set by parser
) -> Result<Vec<Task>, String> {
    let mut final_tasks: Vec<Task> = Vec::new();
    let today = Local::now().date_naive();

    // 1. Index existing tasks by ID for quick lookup and to track seen IDs from markdown
    let mut existing_tasks_map: HashMap<i64, Task> = existing_tasks_vec
        .into_iter()
        .map(|t| (t.id, t))
        .collect();

    // Keep track of IDs present in the new markdown input
    let mut markdown_task_ids: HashSet<i64> = HashSet::new();
    let mut next_display_order = 1;

    // 2. Process tasks from Markdown input
    // This loop handles:
    // - Updates to existing tasks (D.4.2)
    // - Addition of new tasks (D.4.3)
    // - Order of tasks as they appear in Markdown (D.4.5)
    for mut md_task in markdown_tasks_vec {
        markdown_task_ids.insert(md_task.id);
        md_task.display_order = next_display_order;
        next_display_order += 1;

        if let Some(mut existing_task) = existing_tasks_map.remove(&md_task.id) {
            // Task exists, update it based on Markdown content
            // D.4.2: Update fields editable in Markdown
            // D.4.2: Update editable fields from markdown
            existing_task.name = md_task.name;
            existing_task.status = md_task.status;
            existing_task.priority = md_task.priority;
            
            // D.4.7: Attribute deletion - Required keys with nullable values
            // When the key is required but the value can be null (like 'due'), 
            // if it's deleted from MD, we set it to None in the JSON
            existing_task.due = md_task.due;  // Will be None if not in MD
            existing_task.completed = md_task.completed;  // Will be None if not in MD
            
            // created date should not change for existing tasks
            
            // D.4.7: Attribute deletion - Optional keys
            // When optional keys (project, contexts, tags, notes) are deleted from MD,
            // we remove them completely from the JSON (they will be None from the parser)
            existing_task.notes = md_task.notes;  // Will be None if not in MD
            existing_task.project = md_task.project;  // Will be None if not in MD
            existing_task.contexts = md_task.contexts;  // Will be None if not in MD
            existing_task.tags = md_task.tags;  // Will be None if not in MD
            
            // subtasks from markdown should overwrite existing subtasks
            // A more sophisticated subtask merge might be needed in the future
            existing_task.subtasks = md_task.subtasks;

            // D.4.2: updated 日はツール処理日で自動更新
            existing_task.updated = Some(today);
            
            // D.4.5: display_order is set from md_task
            existing_task.display_order = md_task.display_order;

            // D.4.6: JSON固有情報の保護 (extra field) - already part of existing_task, so it's preserved unless overwritten by a more complex rule later.

            final_tasks.push(existing_task);
        } else {
            // New task from Markdown (D.4.3)
            // The parser should have already assigned a provisional ID and created_date.
            // If ID was 0, parser should assign a new unique one.
            // If created was not in MD, parser should use default_created_date.
            // We just need to ensure display_order is set.
            // md_task.created is set by parser.
            // md_task.id is set by parser (auto-incremented if not present or 0).
            md_task.updated = Some(today); // New tasks are also "updated" today
            final_tasks.push(md_task);
        }
    }

    // 4. Handle deletions: tasks in existing_tasks_map were not in markdown_task_ids
    // These are implicitly deleted because they are not added to final_tasks.
    // The spec D.4.4 says: "Markdownからタスク行が削除されたらJSONからも対応タスクを削除（確認なし）。"
    // existing_tasks_map now only contains tasks that were in JSON but not in the new Markdown.

    // 5. Re-sort by display_order to ensure final list is correctly ordered.
    // This is important because we processed markdown tasks sequentially,
    // but new tasks might have been assigned IDs that would place them differently if sorted by ID.
    // The primary sort key for output is the order they appeared in the (new) markdown.
    final_tasks.sort_by_key(|t| t.display_order);
    
    // Ensure all tasks have a valid display_order if any re-ordering or ID generation logic
    // in the parser didn't perfectly align. This is a safeguard.
    for (index, task) in final_tasks.iter_mut().enumerate() {
        task.display_order = (index + 1) as i64;
    }


    Ok(final_tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_model::Task;
    use chrono::NaiveDate;

    fn create_sample_task(id: i64, name: &str, display_order: i64, project: Option<&str>) -> Task {
        Task {
            id,
            name: name.to_string(),
            status: "PENDING".to_string(),
            priority: "N".to_string(),
            created: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            display_order,
            due: None,
            updated: None,
            completed: None,
            project: project.map(String::from),
            contexts: None,
            notes: None,
            tags: None,
            subtasks: None,
            extra: None,
            repeat: None,
        }
    }

    #[test]
    fn test_add_new_task() {
        let existing_tasks = vec![];
        let md_tasks = vec![create_sample_task(1, "New Task", 1, None)];
        let today = Local::now().date_naive();
        let result = apply_changes(existing_tasks, md_tasks, today).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "New Task");
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].display_order, 1);
        assert_eq!(result[0].updated, Some(today));
    }

    #[test]
    fn test_update_existing_task() {
        let today = Local::now().date_naive();
        let existing_tasks = vec![create_sample_task(1, "Old Name", 1, None)];
        let md_tasks = vec![create_sample_task(1, "New Name", 1, Some("ProjectX"))];
        
        let result = apply_changes(existing_tasks, md_tasks, today).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "New Name");
        assert_eq!(result[0].project, Some("ProjectX".to_string()));
        assert_eq!(result[0].updated, Some(today));
        assert_eq!(result[0].display_order, 1);
    }

    #[test]
    fn test_delete_task() {
        let existing_tasks = vec![
            create_sample_task(1, "Task 1", 1, None),
            create_sample_task(2, "Task 2", 2, None),
        ];
        let md_tasks = vec![create_sample_task(1, "Task 1", 1, None)]; // Task 2 is missing
        let today = Local::now().date_naive();
        let result = apply_changes(existing_tasks, md_tasks, today).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    #[test]
    fn test_reorder_tasks() {
        let existing_tasks = vec![
            create_sample_task(1, "Task 1", 1, None),
            create_sample_task(2, "Task 2", 2, None),
        ];
        // Markdown tasks are reordered
        let md_tasks = vec![
            create_sample_task(2, "Task 2", 1, None), // Was 2nd, now 1st
            create_sample_task(1, "Task 1", 2, None), // Was 1st, now 2nd
        ];
        let today = Local::now().date_naive();
        let result = apply_changes(existing_tasks, md_tasks, today).unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, 2);
        assert_eq!(result[0].display_order, 1);
        assert_eq!(result[1].id, 1);
        assert_eq!(result[1].display_order, 2);
    }

    #[test]
    fn test_add_update_delete_reorder_combined() {
        let existing_tasks = vec![
            create_sample_task(1, "Task One Old", 1, None), // To be updated
            create_sample_task(2, "Task Two Delete", 2, None), // To be deleted
            create_sample_task(3, "Task Three Old", 3, None), // To be reordered and updated
        ];
        let md_tasks = vec![
            create_sample_task(4, "Task Four New", 1, Some("NewProj")), // New task, first
            create_sample_task(3, "Task Three New Name", 2, None), // Reordered (was 3rd, now 2nd) and updated
            create_sample_task(1, "Task One New Name", 3, None), // Updated (was 1st, now 3rd)
        ];
        let today = Local::now().date_naive();
        let result = apply_changes(existing_tasks, md_tasks, today).unwrap();

        assert_eq!(result.len(), 3);

        // Task Four New (ID 4)
        assert_eq!(result[0].id, 4);
        assert_eq!(result[0].name, "Task Four New");
        assert_eq!(result[0].project, Some("NewProj".to_string()));
        assert_eq!(result[0].display_order, 1);
        assert_eq!(result[0].updated, Some(today));


        // Task Three New Name (ID 3)
        assert_eq!(result[1].id, 3);
        assert_eq!(result[1].name, "Task Three New Name");
        assert_eq!(result[1].display_order, 2);
        assert_eq!(result[1].updated, Some(today));


        // Task One New Name (ID 1)
        assert_eq!(result[2].id, 1);
        assert_eq!(result[2].name, "Task One New Name");
        assert_eq!(result[2].display_order, 3);
        assert_eq!(result[2].updated, Some(today));
    }
    
    #[test]
    fn test_preserve_extra_field_on_update() {
        let mut task1_existing = create_sample_task(1, "Task 1 Old", 1, None);
        let mut extra_data = HashMap::new();
        extra_data.insert("custom_key".to_string(), serde_json::json!("custom_value"));
        task1_existing.extra = Some(extra_data);

        let existing_tasks = vec![task1_existing];
        let md_tasks = vec![create_sample_task(1, "Task 1 New", 1, Some("ProjectX"))]; // No extra field in MD
        
        let today = Local::now().date_naive();
        let result = apply_changes(existing_tasks, md_tasks, today).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Task 1 New");
        assert!(result[0].extra.is_some());
        assert_eq!(result[0].extra.as_ref().unwrap().get("custom_key").unwrap(), &serde_json::json!("custom_value"));
        assert_eq!(result[0].updated, Some(today));
    }
}
