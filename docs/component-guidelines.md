# Component Guidelines

## Overview

This document outlines the standards and best practices for developing components in the RustyGPT frontend.

## Component Structure

### File Organization

- One component per file
- Components organized by feature/domain
- Reusable UI components in a dedicated components directory
- Tests alongside component files

### Naming Conventions

- PascalCase for component names
- snake_case for function and variable names
- Event handlers named after the event (on_click, on_submit, etc.)

## Component Design Principles

### Props

- All customizable aspects should be accessible via props
- Default values for optional props
- Clear documentation of prop types and purposes
- Support for custom CSS classes on all components

```rust
#[derive(Properties, PartialEq)]
pub struct ButtonProps {
    #[prop_or_default]
    pub children: Children,
    #[prop_or_default]
    pub onclick: Callback<MouseEvent>,
    #[prop_or_default]
    pub variant: ButtonVariant,
    #[prop_or_default]
    pub size: ButtonSize,
    #[prop_or_default]
    pub class: Classes,
    #[prop_or_default]
    pub disabled: bool,
}
```

### Composition

- Prefer composition over inheritance
- Design components to be combined with other components
- Use children prop for flexible content insertion

### CSS Handling

- Use Tailwind CSS and DaisyUI for styling
- Support custom classes via props
- Use the `classes!` macro to combine classes

```rust
let combined_classes = classes!(
    "base-class",
    if active { "active-class" } else { "" },
    props.class.clone()
);
```

## Accessibility Requirements

### Semantic HTML

- Use appropriate HTML elements for their intended purpose
- Maintain proper heading hierarchy
- Use buttons for actions, links for navigation

### ARIA Attributes

- Include appropriate ARIA roles, states, and properties
- Ensure proper labeling for all interactive elements
- Test with screen readers

### Keyboard Navigation

- Ensure all interactive elements are keyboard accessible
- Implement proper tab order
- Add visible focus indicators

### Focus Management

- Trap focus in modals and dialogs
- Return focus appropriately after modal dismissal
- Set proper tabindex values

## Performance Considerations

### Memoization

- Use memoization for expensive computations
- Prevent unnecessary re-renders

### Event Handling

- Implement debouncing for input-heavy components
- Avoid anonymous functions in render methods when possible

### Rendering Optimization

- Implement virtualization for large lists
- Lazy load components when appropriate

## Testing Requirements

### Unit Tests

- Test component logic and state
- Verify props handling
- Test event handlers

### Component Tests

- Test component rendering
- Verify DOM structure
- Test component interactions

### Accessibility Tests

- Verify ARIA attributes
- Test keyboard navigation
- Check screen reader compatibility

## Documentation Requirements

### Component Documentation

- Document all props with types and descriptions
- Include usage examples
- Document accessibility considerations
- Note any performance considerations

## Internationalization (i18n)

### Key Principles

- **No hardcoded strings**: All user-facing text must use i18n
- **Complete key coverage**: Ensure all languages support all keys
- **Natural language**: Use translations that feel natural to native speakers
- **Component integration**: Design components to support i18n seamlessly

### i18n Integration

#### Basic Text Translation

```rust
let (i18n, ..) = use_translation();
html! {
    <p>{ i18n.t("some.translation.key") }</p>
}
```

#### Text with Variables

```rust
let (i18n, ..) = use_translation();
html! {
    <p>{ i18n.t("greeting.with_name", &[("name", &user_name)]) }</p>
}
```

#### Handling Plurals

```rust
let (i18n, ..) = use_translation();
let count = 5;
html! {
    <p>{ i18n.t_plural("items.count", count, &[("count", &count.to_string())]) }</p>
}
```

### Translation File Structure

```json
{
  "common": {
    "submit": "Submit",
    "cancel": "Cancel",
    "save": "Save",
    "delete": "Delete"
  },
  "auth": {
    "login": "Log In",
    "logout": "Log Out",
    "sign_up": "Sign Up"
  },
  "dashboard": {
    "title": "Dashboard",
    "refresh": "Refresh Data",
    "stats": {
      "users": "Total Users",
      "revenue": "Revenue"
    },
    "charts": {
      "user_growth": "User Growth",
      "user_growth_desc": "Monthly user acquisition",
      "revenue": "Revenue",
      "revenue_desc": "Monthly revenue"
    }
  }
}
```

### Testing i18n

- Test with all supported languages
- Verify string expansion with variables
- Check for missing translations
- Ensure proper pluralization

## Example Components

### Basic Button Component

```rust
use i18nrs::yew::use_translation;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Outline,
    Text,
}

#[derive(Clone, PartialEq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

#[derive(Properties, PartialEq)]
pub struct ButtonProps {
    #[prop_or_default]
    pub children: Children,
    #[prop_or_default]
    pub onclick: Callback<MouseEvent>,
    #[prop_or(ButtonVariant::Primary)]
    pub variant: ButtonVariant,
    #[prop_or(ButtonSize::Medium)]
    pub size: ButtonSize,
    #[prop_or_default]
    pub class: Classes,
    #[prop_or(false)]
    pub disabled: bool,
    // i18n key for button text (alternative to children)
    #[prop_or_default]
    pub text_key: Option<String>,
}

#[function_component(Button)]
pub fn button(props: &ButtonProps) -> Html {
    let (i18n, ..) = use_translation();

    let base_classes = "btn";
    let variant_classes = match props.variant {
        ButtonVariant::Primary => "btn-primary",
        ButtonVariant::Secondary => "btn-secondary",
        ButtonVariant::Outline => "btn-outline",
        ButtonVariant::Text => "btn-ghost",
    };
    let size_classes = match props.size {
        ButtonSize::Small => "btn-sm",
        ButtonSize::Medium => "",
        ButtonSize::Large => "btn-lg",
    };

    let combined_classes = classes!(
        base_classes,
        variant_classes,
        size_classes,
        props.class.clone()
    );

    html! {
        <button
            class={combined_classes}
            onclick={props.onclick.clone()}
            disabled={props.disabled}
            type="button"
        >
            {
                if let Some(key) = &props.text_key {
                    html! { { i18n.t(key) } }
                } else {
                    html! { { for props.children.iter() } }
                }
            }
        </button>
    }
}
```
