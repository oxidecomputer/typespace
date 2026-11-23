# Notes for AI coding agents

- Read CONTRIBUTING.md before making changes.
- When opening a PR, fill in every section of .github/pull_request_template.md.
  Don't replace it with your own format or pass a free-form `--body`.
- "Why are you here?": ask the human. Don't invent a motivation.
- "AI disclosure": name the tool and model, and say what it did.
- If you expect no human to have reviewed a commit before you open the PR that
  carries it, end the commit message with the trailer `Human-Review: required`.
- If any of the PR's commits has not been reviewed by a human when you open it,
  end the PR description with the line `<!-- requires human review -->`.
- These tell the maintainers the change still needs a human reader, and they
  will be that reader.
- Only describe tests you actually ran.
- Fixing hypothetical issues wastes reviewer time; if the human didn't actually
  encounter the bug being fixed, confirm that they still want to proceed.
