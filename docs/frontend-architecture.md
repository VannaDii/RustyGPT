# Frontend Architecture

## Overview

This document outlines the architecture of the RustyGPT frontend, following the rewrite to implement a DaisyUI admin dashboard template using Yew and related Rust-based web frameworks.

## Goals

- Replicate the structure and functionality of the daisyui-admin-dashboard-template
- Create a component-driven architecture following atomic design principles
- Achieve high performance with optimized WebAssembly
- Ensure accessibility compliance (WCAG 2.2 AA)
- Support responsive design across devices
- Maintain clean, maintainable code

## Core Architecture

### Component Architecture

We follow atomic design methodology, breaking UI into:

- **Atoms**: Basic building blocks (buttons, inputs, icons)
- **Molecules**: Simple component combinations (form fields, menu items)
- **Organisms**: Complex components (sidebars, headers, tables)
- **Templates**: Page layouts
- **Pages**: Specific instances of templates

All components:

- Accept props for customization
- Support custom CSS classes
- Implement proper accessibility attributes
- Include comprehensive tests and documentation

### State Management

We use Yewdux for state management with these stores:

- **AuthStore**: Authentication state, user info, tokens
- **ThemeStore**: Theme preferences and settings
- **UIStore**: UI state (sidebar open/closed, active page)
- **DataStore**: Application data caching and management

### Routing

We use Yew Router with:

- Route-level code splitting
- Protected routes
- Suspense with skeleton loaders

## Technical Implementation

### Performance Optimizations

- Sub-1MB initial WASM payload
- Route-level code splitting
- wasm-opt with -Oz optimization
- wee_alloc for smaller memory footprint
- Lazy loading and responsive images
- Virtualization for large lists
- Memoization to prevent unnecessary re-renders

### API Client

Our API client implementation includes:

- Automatic retry mechanism
- Request caching
- Structured error handling
- Request cancellation
- Authentication token management

### Accessibility

We ensure WCAG 2.2 AA compliance through:

- Semantic HTML
- ARIA attributes
- Keyboard navigation
- Focus management
- Color contrast compliance
- Screen reader compatibility

### Theme System

Our theme system includes:

- Light and dark modes
- System preference detection
- User preference override
- Consistent theme variables

## Implementation Phases

1. **Foundation**: Project setup, core components, state management
2. **Structure**: Layout framework, navigation, authentication
3. **Features**: Dashboard components, form system, feedback systems
4. **Refinement**: Performance optimization, accessibility audit, cross-browser testing
5. **Documentation and Delivery**: Component documentation, final QA

## Performance Benchmarks

- **Lighthouse Scores**: ≥90 for Performance, Best Practices, and SEO; 100 for Accessibility
- **Core Web Vitals**:
  - First Contentful Paint (FCP): ≤1.8s
  - Largest Contentful Paint (LCP): ≤2.5s
  - Time to Interactive (TTI): ≤3s
  - Total Blocking Time (TBT): ≤100ms
- **Bundle Size**: ≤500KB compressed WASM bundle

## Backend Integration

The frontend integrates with our backend for:

- Authentication (GitHub, future Apple)
- User data and preferences
- Application-specific APIs

## Future Enhancements

- Apple authentication
- Analytics integration
- Additional dashboard features

## Resources

- [Component Guidelines](./component-guidelines.md)
- [State Management Overview](./state-management.md)
- [Frontend Development Guide](./frontend-development.md)
