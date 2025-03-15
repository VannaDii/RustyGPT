# State Management Overview

## Introduction

This document outlines the state management approach for the RustyGPT frontend, using Yewdux for global state management.

## Core Principles

- Keep state as close to components as possible
- Use global state only when necessary
- Structure state for minimizing re-renders
- Implement memoization for performance
- Create clear actions for state updates

## Yewdux Store Structure

### AuthStore

Manages authentication and user information.

```rust
#[derive(Default, Clone, PartialEq, Store)]
pub struct AuthStore {
    pub authenticated: bool,
    pub user: Option<User>,
    pub token: Option<String>,
    pub loading: bool,
    pub error: Option<String>,
}

pub enum AuthAction {
    SetAuthenticated(bool),
    SetUser(Option<User>),
    SetToken(Option<String>),
    SetLoading(bool),
    SetError(Option<String>),
    Logout,
}
```

### ThemeStore

Manages theme preferences and settings.

```rust
#[derive(Default, Clone, PartialEq, Store)]
pub struct ThemeStore {
    pub theme: Theme,
    pub use_system_preference: bool,
}

pub enum Theme {
    Light,
    Dark,
}

pub enum ThemeAction {
    SetTheme(Theme),
    SetUseSystemPreference(bool),
    ToggleTheme,
}
```

### UIStore

Manages UI state such as sidebar state, active page, etc.

```rust
#[derive(Default, Clone, PartialEq, Store)]
pub struct UIStore {
    pub sidebar_open: bool,
    pub right_sidebar_open: bool,
    pub active_page: String,
    pub loading: HashMap<String, bool>,
}

pub enum UIAction {
    SetSidebarOpen(bool),
    SetRightSidebarOpen(bool),
    SetActivePage(String),
    SetLoading(String, bool),
    ToggleSidebar,
    ToggleRightSidebar,
}
```

### DataStore

Manages application data caching and management.

```rust
#[derive(Default, Clone, PartialEq, Store)]
pub struct DataStore {
    pub data: HashMap<String, Json>,
    pub last_updated: HashMap<String, DateTime<Utc>>,
    pub loading: HashMap<String, bool>,
    pub errors: HashMap<String, String>,
}

pub enum DataAction {
    SetData(String, Json),
    RemoveData(String),
    SetLoading(String, bool),
    SetError(String, Option<String>),
    ClearAll,
}
```

## State Usage Best Practices

### Component State

- Use `use_state` for component-local state
- Use `use_reducer` for more complex component state
- Minimize state updates during renders

### Global State

- Use Yewdux's `use_store` to access stores
- Use Yewdux's `Dispatch` to update stores
- Create selector functions to reduce re-renders

### Memoization

- Use `use_memo` to memoize expensive computations
- Use `use_callback` to memoize callbacks

## API Integration

### Request State Management

- Track loading state for requests
- Handle errors consistently
- Cache responses where appropriate
- Implement automatic retries for failed requests

### Authentication Flow

- Manage tokens in AuthStore
- Implement automatic token refresh
- Handle authentication errors

## Examples

### Using Local State

```rust
#[function_component(Counter)]
pub fn counter() -> Html {
    let (i18n, ..) = use_translation();
    let counter = use_state(|| 0);
    let onclick = {
        let counter = counter.clone();
        Callback::from(move |_| {
            counter.set(*counter + 1);
        })
    };

    html! {
        <div>
            <p>{ i18n.t("counter.value", &[("count", &counter.to_string())]) }</p>
            <button {onclick}>{ i18n.t("counter.increment") }</button>
        </div>
    }
}
```

### Using Global State

```rust
#[function_component(ThemeToggle)]
pub fn theme_toggle() -> Html {
    let (i18n, ..) = use_translation();
    let (theme, dispatch) = use_store::<ThemeStore>();
    let onclick = dispatch.apply_callback(|_| ThemeAction::ToggleTheme);

    let theme_text_key = if theme.theme == Theme::Dark {
        "theme.switch_to_light"
    } else {
        "theme.switch_to_dark"
    };

    html! {
        <button onclick={onclick}>
            { i18n.t(theme_text_key) }
        </button>
    }
}
```

### Using Selectors

```rust
#[function_component(UserInfo)]
pub fn user_info() -> Html {
    let (i18n, ..) = use_translation();
    let username = use_selector(|state: &AuthStore| {
        state.user.as_ref().map(|u| u.username.clone()).unwrap_or_default()
    });

    html! {
        <div>
            { i18n.t("user.greeting", &[("username", &username)]) }
        </div>
    }
}
```
