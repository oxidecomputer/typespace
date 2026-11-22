# Notes for AI coding agents

- Read CONTRIBUTING.md before making changes.
- When opening a PR, fill in every section of .github/pull_request_template.md.
  Don't replace it with your own format or pass a free-form `--body`.
- "Why are you here?": ask the human. Don't invent a motivation.
- "AI disclosure": name the tool and model, and say what it did.
- If no human has read the diff when you open the PR, end every commit message
  with the trailer `Human-Review: required` and end the PR description with
  the line `<!-- requires human review -->`. That tells the maintainers the
  change still needs a human reader, and they will be that reader.
- Only describe tests you actually ran.
- Fixing hypothetical issues wastes reviewer time; if the human didn't actually
  encounter the bug being fixed, confirm that they still want to proceed.
