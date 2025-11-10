use std::collections::HashSet;

use uuid::Uuid;

use crate::models::chat::MessageView;

/// Builds ordered context slices for thread-aware completions.
#[derive(Debug, Clone)]
pub struct ThreadContextBuilder {
    messages: Vec<MessageView>,
}

impl ThreadContextBuilder {
    /// Creates a new builder with messages ordered by depth-first traversal.
    #[must_use]
    pub fn new(mut messages: Vec<MessageView>) -> Self {
        messages.sort_by(|a, b| match a.depth.cmp(&b.depth) {
            std::cmp::Ordering::Equal => a.path.cmp(&b.path),
            ordering => ordering,
        });
        Self { messages }
    }

    /// Returns the root message (thread head) if present.
    #[must_use]
    pub fn root(&self) -> Option<&MessageView> {
        self.messages.iter().find(|msg| msg.id == msg.root_id)
    }

    /// Returns the ancestor chain for the provided parent message identifier.
    /// The chain is ordered from root to the parent inclusive.
    #[must_use]
    pub fn ancestor_chain(&self, parent_id: Uuid) -> Vec<MessageView> {
        let Some(parent) = self.messages.iter().find(|msg| msg.id == parent_id) else {
            return Vec::new();
        };

        let prefixes = path_prefix_set(&parent.path);

        self.messages
            .iter()
            .filter(|msg| prefixes.contains(&msg.path))
            .cloned()
            .collect()
    }

    /// Returns the full thread in depth-first order, truncated optionally by depth.
    #[must_use]
    pub fn full_thread(&self, max_depth: Option<i32>) -> Vec<MessageView> {
        max_depth.map_or_else(
            || self.messages.clone(),
            |limit| {
                self.messages
                    .iter()
                    .filter(|msg| msg.depth <= limit)
                    .cloned()
                    .collect()
            },
        )
    }

    /// Returns the immediate children of a given parent ordered by path.
    #[must_use]
    pub fn children(&self, parent_id: Uuid) -> Vec<MessageView> {
        let Some(parent) = self.messages.iter().find(|msg| msg.id == parent_id) else {
            return Vec::new();
        };

        self.messages
            .iter()
            .filter(|msg| msg.parent_id == Some(parent.id))
            .cloned()
            .collect()
    }
}

fn path_prefix_set(path: &str) -> HashSet<String> {
    let segments: Vec<&str> = path.split('.').collect();
    let mut prefixes = HashSet::with_capacity(segments.len());
    let mut current = String::new();
    for (idx, segment) in segments.iter().enumerate() {
        if idx == 0 {
            current.clear();
        } else {
            current.push('.');
        }
        current.push_str(segment);
        prefixes.insert(current.clone());
    }
    prefixes
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_message(id: Uuid, parent_id: Option<Uuid>, path: &str, depth: i32) -> MessageView {
        MessageView {
            id,
            conversation_id: Uuid::new_v4(),
            root_id: parent_id.unwrap_or(id),
            parent_id,
            author_user_id: None,
            role: crate::models::chat::MessageRole::User,
            content: String::new(),
            path: path.to_string(),
            depth,
            created_at: crate::models::timestamp::Timestamp(Utc::now()),
        }
    }

    #[test]
    fn ancestor_chain_returns_ordered_messages() {
        let root_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let root = sample_message(root_id, None, "mroot", 1);
        let child = sample_message(child_id, Some(root_id), "mroot.mchild", 2);

        let builder = ThreadContextBuilder::new(vec![child, root]);
        let ancestors = builder.ancestor_chain(child_id);

        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0].id, root_id);
        assert_eq!(ancestors[1].id, child_id);
    }

    #[test]
    fn children_returns_direct_descendants() {
        let root_id = Uuid::new_v4();
        let child_one = Uuid::new_v4();
        let child_two = Uuid::new_v4();

        let root = sample_message(root_id, None, "mroot", 1);
        let child1 = sample_message(child_one, Some(root_id), "mroot.m1", 2);
        let child2 = sample_message(child_two, Some(root_id), "mroot.m2", 2);

        let builder = ThreadContextBuilder::new(vec![child1, child2, root]);
        let children = builder.children(root_id);

        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|msg| msg.id == child_one));
        assert!(children.iter().any(|msg| msg.id == child_two));
    }
}
