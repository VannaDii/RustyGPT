use std::collections::HashMap;

#[test]
fn test_task_name_uniqueness() {
    // Test that task names are properly made unique when there are duplicates

    // Create a list of base task names
    let base_names = vec![
        "task1".to_string(),
        "task1".to_string(), // Duplicate
        "task2".to_string(),
        "task2".to_string(), // Duplicate
        "task2".to_string(), // Another duplicate
        "task3".to_string(),
    ];

    // Generate unique names using the algorithm from main.rs
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    let mut unique_names = Vec::new();

    for base_name in base_names {
        let count = name_counts.entry(base_name.clone()).or_insert(0);
        let unique_name = if *count > 0 {
            format!("{}#{}", base_name, *count)
        } else {
            base_name.clone()
        };
        *count += 1;
        unique_names.push(unique_name);
    }

    // Verify the results
    assert_eq!(unique_names[0], "task1");
    assert_eq!(unique_names[1], "task1#1");
    assert_eq!(unique_names[2], "task2");
    assert_eq!(unique_names[3], "task2#1");
    assert_eq!(unique_names[4], "task2#2");
    assert_eq!(unique_names[5], "task3");

    // Make sure all names are unique
    let mut unique_set = std::collections::HashSet::new();
    for name in &unique_names {
        assert!(unique_set.insert(name));
    }
    assert_eq!(unique_set.len(), unique_names.len());
}
