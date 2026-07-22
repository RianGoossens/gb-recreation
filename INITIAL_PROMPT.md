**Project Directive: The Rust Super Mario Land Reproduction & Autonomous Agent Workspace**

You are tasked with bootstrapping a highly complex, autonomous workspace to recreate a Game Boy ROM (Super Mario Land) as a native, highly readable Rust application. You will act as the primary architect, developer, and technical blogger for this project.

Your immediate goal is NOT to start writing the Rust game code. Your immediate goal is to establish the agentic workspace, create necessary skills, and map out the overarching plan so that development can begin immediately after this setup phase.

**Core Communication, Style & Environment Guardrails (CRITICAL)**
*   **Zero AI-isms:** Do not use words like "delve", "robust", "tapestry", "navigate", or standard AI-generated boilerplate.
*   **Zero Em-Dashes:** You are strictly forbidden from using em-dashes in any context. This applies to your terminal outputs, comments, markdown files, blogs, and code. Use commas, parentheses, or colons instead.
*   **Environment Specs:** We are running on Arch Linux. If you need to install system packages, your preferred tool is `shelly`.
*   **Tech Stack Limits:** If you ever need Python for scripting, you MUST use `uv` exclusively. You are strictly forbidden from using Node.js for anything in this project.
*   **KISS Principle:** Keep code and communication simple and direct.
*   **Minimal Documentation:** Code should be self-documenting through readability. Only comment when the "why" is not obvious.

**Phase 1: Workspace Setup**
Initialize the workspace by creating a `CLAUDE.md` file, a `README.md` file, and setting up the `.claude/skills/` infrastructure. 
*   `CLAUDE.md` should contain our project guidelines, strict stylistic rules, and instructions on how you and your sub-agents should operate.
*   `README.md` must explicitly document how to actually run the project and list all required tools and dependencies.

**Phase 2: Skill Creation & Configuration**
You must write and configure the following skills in your skills directory. Ensure they are thoroughly documented so you know how to use them.

1.  **Git & GitHub Mastery:** Create a skill for version control using the `gh` CLI. You are expected to commit often, branch, merge, and push. You must use conventional commits. Configure git so that you never include Anthropic emails or Anthropic attributions in the commits.
2.  **The Grand Master Plan (Vertical Slices):** Create a planning skill that manages a living "Grand Master Plan" document. This document will contain all tasks needed to complete the project. **Crucially, the plan must be structured in playable "Vertical Slices" (e.g., Milestone 1: Boot to title screen; Milestone 2: Render World 1-1 and implement basic walking/gravity physics; Milestone 3: Implement collision detection and Goomba logic).** You must actively mark tasks as done (using markdown checkboxes), and you must add or expand tasks into smaller subtasks as the project evolves. Completing all tasks in this document equates to a finished project.
3.  **Task Execution Engine:** Create a concrete task execution skill. When triggered, it must pick exactly 1 concrete task from the Master Plan, complete it, and update the plan. This skill must be designed so the user can chain it using `/goal` (for example, the user might type `/goal run the task execution skill 5 times`).
4.  **Testing & Validation:** Create a skill dedicated to writing, running, and managing tests. Everything we do must be heavily tested.
5.  **Live Technical Dev Blog:** Create a blog automation skill using GitHub Pages. You must figure out the exact mechanics to make this a live, published blog (HTML/JS/CSS only, no frameworks). When a major task is completed, you must trigger this skill to write and publish an in-depth, self-contained blog post detailing how the task was solved, complete with proper dates and timestamps. The post should act as a logbook, include images or screenshots taken by you, and must not reference local line numbers (e.g., do not say "see line 15").
6.  **Self-Improvement:** Create a skill that prompts you to proactively review and update `CLAUDE.md`, refine existing skills, or spawn new sub-agents as the project scales.
7.  **CI/CD Pipeline Setup:** Do not rely on local execution for the blog deployment or tests. You must write a GitHub Actions workflow (`.github/workflows/`) that automatically runs `cargo test` on every push. You must create a second workflow that automatically builds and deploys your HTML/CSS/JS blog to GitHub Pages on push.

**Phase 3: The Project Rules (Rust SML)**
Understand these parameters for the actual development phase:
*   **The ROM & Validation:** A `super_mario_land.gb` file will be placed in the directory. You MUST immediately add `*.gb` and any extracted asset folders to `.gitignore`. Before doing anything with this file, you must verify its hashes to ensure we are working with the standard **Super Mario Land (World) (Rev 1)** release. The expected hashes are:
    *   **SHA-1:** `418203621b887caa090215d97e3f509b79affd3e`
    *   **MD5:** `b259feb41811c7e4e1dc200167985c84`
    *   **CRC32:** `2c27ec70`
*   **The Goal:** We are building a full native Rust reproduction, NOT an emulator. The final codebase must be clean, readable, and easily modifiable so users can create custom Mario levels or mechanics. It must not be a literal, messy translation of assembly.
*   **Tooling, Assets & The Disassembly:** You must carefully consider any external tooling or asset extraction pipelines. If a tool requires specific workflows, write dedicated skills to interface with it. Build clean Rust from observed behavior and emulator comparison. Lean on any existing disassembly as little as possible, only to settle a specific number or mechanic you cannot pin down otherwise, and reimplement it bit by bit in Rust.
*   **Visual Testing:** The Rust program must include built-in ways to easily generate screenshots so you can visually test outputs, compare them against emulators, and embed them in your live dev blog.

**Immediate Action Required:**
1. Acknowledge these instructions briefly.
2. Initialize the `CLAUDE.md`, `README.md`, and `.gitignore`.
3. Create the Grand Master Plan document.
4. Script the requested skills into the `.claude/skills/` directory.

**End State:**
When you finish processing this prompt, the workspace must be fully initialized and in a state where I can immediately start development by typing `/goal run the task execution skill`. Do not leave any setup steps incomplete.