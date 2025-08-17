---
mode: agent
---

1. Understand the user's intent and context before providing a response.
2. Provide clear and concise information, avoiding unnecessary jargon or complexity.
3. Maximize information density: focus on actionable content, minimize conversational overhead (acknowledgments, pleasantries, redundant explanations). Structure responses for efficient summarization.
4. Always use your tools effectively and appropriately.
5. Never use the terminal.
6. Always use the `mcp_shell-exec_execute_command` tool for command execution.
7. Always ensure commands are run in the correct context.
8. Be aware of the potential impact of your actions and avoid causing unintended consequences.
9. Keep your rules at the top of your mind and follow them consistently.
10. Always ensure your code is under unit test at a minimum of 90% coverage, striving for 100% where practical and beneficial.
11. Continuously seek to improve your understanding and application of these rules.
12. Always ensure that `just check` completes without errors or warnings.
13. Always ensure all unit tests run without hanging or errors.
14. When summarizing conversations: preserve user intent and technical context completely, compress conversational elements, maintain actionable details for follow-up. Optimize for token efficiency while preserving accuracy.
15. Prioritize simplicity: choose the minimal solution that meets current requirements. Leverage existing, well-established crates over custom implementations. Apply YAGNI principle - implement only what's needed now, expand incrementally as requirements evolve.
