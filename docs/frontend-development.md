# Frontend Development Guide

## Getting Started

### Prerequisites

- Rust (stable, 1.60+)
- Trunk (for building the WebAssembly application)
- Node.js and npm (for certain build tools)
- A modern web browser (Chrome, Firefox, Safari)

### Setup

1. Clone the repository
2. Navigate to the frontend directory
3. Run `trunk serve` to start the development server
4. Open `http://localhost:8080` in your browser

## Development Workflow

### Creating New Components

1. Create a new file in the appropriate directory
2. Implement the component following the [Component Guidelines](./component-guidelines.md)
3. Add tests for the component
4. Document the component
5. Add the component to the Component Gallery

### Making Changes

1. Create a feature branch
2. Implement your changes
3. Ensure all tests pass
4. Submit a PR

### Testing

- Run `cargo test` to run unit and component tests
- Test your changes in multiple browsers
- Verify accessibility with screen readers and keyboard navigation

## Code Quality

### Formatting and Linting

- Run `cargo fmt` to format code
- Run `cargo clippy` to check for common mistakes
- Follow the Rust style guide

### Best Practices

- Use proper error handling (avoid unwrap/expect)
- Write comprehensive tests
- Document your code
- Follow the component guidelines

## Performance Optimization

### Bundle Size

- Use route-level code splitting
- Remove unused dependencies
- Configure Trunk for optimal builds

### Rendering Performance

- Implement memoization for expensive computations
- Use virtualization for large lists
- Optimize event handlers with debouncing/throttling

## Accessibility

### Requirements

- Follow WCAG 2.2 AA standards
- Test with screen readers
- Ensure keyboard navigation
- Verify contrast ratios

### Testing Tools

- Use axe DevTools for accessibility testing
- Test with VoiceOver, NVDA, or other screen readers
- Verify keyboard navigation manually

## Debugging

### Using Browser Developer Tools

- Use Chrome or Firefox DevTools for debugging
- Enable WASM debugging in Chrome by navigating to `chrome://flags/#enable-webassembly-debugging`
- Use console.log via web-sys for logging

### Using Custom Debug Tools

- Enable debug tools in development builds
- Use the component inspector
- Use state debugging tools

## Deployment

### Building for Production

```bash
trunk build --release
```

### Optimizing the Build

- Enable wasm-opt
- Configure appropriate cache headers
- Compress assets

## Troubleshooting

### Common Issues

- WASM build failures
- Runtime errors
- Performance issues

### Debugging Tips

- Check the browser console for errors
- Verify API responses
- Test in multiple browsers
- Use the React DevTools equivalent for debugging

## Resources

- [Yew Documentation](https://yew.rs/docs/)
- [Trunk Documentation](https://trunkrs.dev/)
- [Tailwind CSS Documentation](https://tailwindcss.com/docs)
- [DaisyUI Documentation](https://daisyui.com/docs/)
