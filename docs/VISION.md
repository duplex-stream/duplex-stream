# Duplex Stream

> Shared understanding that persists and adapts. Open source.

---

## The Problem

Software development has an understanding problem.

Every day, teams make thousands of decisions. Why did we choose this architecture? What constraints shaped that API? Who decided we'd use this pattern, and what alternatives did they consider? This reasoning lives in conversations, then vanishes. The code survives. The understanding doesn't.

AI is making this worse, not better.

"Vibe coding" produces working software at unprecedented speed. But it also produces software that no one understands. The AI doesn't know why it made the choices it made. The human directing it was focused on outcomes, not reasoning. Three months later, someone needs to modify that code. The modification breaks something. No one knows why it was built that way in the first place.

We're heading toward a future where humans and AI can build things together faster than ever, but can't maintain what they've built. The bottleneck isn't creation anymore. It's comprehension.

---

## The Paradigm

### Shared Cognition, Not Artificial General Intelligence

The "agents will replace everyone" crowd and the "AI is just autocomplete" crowd are both wrong. The future that actually works is shared cognition. Humans and AI building understanding together.

This isn't a compromise position. It's a recognition that understanding is inherently collaborative. It emerges from the interaction between perspectives. A human alone misses things. An AI alone hallucinates things. But a human and AI thinking together, with their reasoning visible to each other, catch errors that neither would catch alone.

The goal isn't to replace human cognition with artificial cognition. It's to create systems where understanding can be built, preserved, and evolved by both humans and AI working together.

### Understanding Over Activity

Most tools optimize for activity. More commits. More messages. More output. Duplex stream optimizes for understanding. 

Activity that doesn't produce understanding is waste. Understanding that enables effective activity is leverage. A single well-documented decision can save hundreds of hours of re-litigation. A clear dependency graph can prevent cascading failures before they happen.

We measure success not by how much was done, but by how well what was done is understood.

### Participant Parity

In a duplex stream, humans and AI are both participants. They have different capabilities, different strengths, different failure modes. But they participate in the same shared understanding. Neither is privileged. Neither is hidden.

When an AI makes a decision, that decision is visible, attributed, and queryable. When a human makes a decision, the same is true. The understanding layer doesn't distinguish between "human knowledge" and "AI knowledge." It distinguishes between well-supported conclusions and poorly-supported ones, between active decisions and superseded ones, between high-confidence extractions and tentative interpretations.

### Transparency as Foundation

You cannot build shared understanding on hidden foundations.

If you don't know what something costs, you can't reason about tradeoffs. If you don't know who decided something, you can't evaluate their reasoning. If you don't know what alternatives were considered, you can't know whether the choice was well-made.

Duplex stream makes everything visible: costs, decisions, reasoning, participants, dependencies. Not because transparency is virtuous, but because understanding is impossible without it.

### Stated Reasoning, Not True Reasoning

An epistemological clarification: when we extract "reasoning" from conversations, we're capturing what was said about why something happened, not a guaranteed causal explanation.

This applies to both humans and AI. When a human explains a decision, they construct a post-hoc narrative. Memory is reconstructive. The "reasons" given may not reflect the actual cognitive process. When an AI explains a decision, it generates a plausible narrative that wasn't derived from introspective access to its weights.

This isn't a flaw to work around. It's the nature of reasoning as communication.

What we capture is still valuable:
- Considerations that were explicitly discussed
- Constraints that were acknowledged at decision time
- Alternatives that were articulated and rejected
- The narrative that participants believed explained their choice

Stated reasoning at decision time is more useful than no reasoning at all, and more honest than reconstructed reasoning months later. The understanding layer captures what was said, attributes who said it, and lets consumers evaluate credibility themselves.

---

## The Protocol

### Core Concepts

Understanding in duplex stream is structured around five primitives:

**Concepts** are the things we're reasoning about. A concept might be a technical term, an architectural pattern, a business requirement, a user need. Concepts have definitions that evolve as understanding deepens. They have relationships to other concepts. They appear in conversations and decisions.

**Relationships** connect concepts to each other. A relationship has a type (supports, contradicts, implements, depends-on, supersedes) and a direction. Relationships form graphs. These graphs are the structure of understanding.

**Decisions** are commitments. A decision chooses between alternatives based on reasoning. It has a status (active, superseded, tentative). It depends on other decisions. It produces artifacts. Decisions are the load-bearing elements of understanding. Everything else supports them.

**Participants** are the entities that contribute to understanding. Humans, AI assistants, integrations, automated systems. Every contribution is attributed to a participant. Participants have roles and permissions within workspaces.

**Events** are the atomic units of change. Something was added, modified, connected, superseded. Events are immutable. They form the complete history of how understanding evolved. Events enable replay, debugging, audit, and temporal queries.

### Temporal Versioning

Everything in the understanding layer is versioned through events. Changes don't overwrite. They append.

**How it works:**
- Concept definition added (timestamp, content, author)
- Concept definition updated (timestamp, new content, author)
- Relationship created (timestamp, type, endpoints, author)
- Relationship removed (timestamp, author)
- Decision status changed (timestamp, old status, new status, author)

**Temporal queries:**
- "What is X?" → current definition
- "What was X when decision Y was made?" → definition as of decision timestamp
- "How has our understanding of X evolved?" → all definition events, chronological

**Why this matters:**
When you ask "why did we choose event sourcing?" and the team's understanding of "event sourcing" has shifted three times, the system can show which definition applied when. The decision references the conversation where it was made. The conversation has a timestamp. The query finds the concept definition active at that timestamp.

### The Understanding Layer

The understanding layer is the persistent, queryable representation of shared cognition. It's not documentation (static, quickly outdated). It's not a knowledge graph (just structure, no reasoning). It's not a wiki (pages, not relationships).

The understanding layer is:

- **Extracted**: Decisions and concepts are pulled from conversations, not manually entered
- **Connected**: Everything links to everything else through typed relationships
- **Temporal**: Understanding evolves; the layer preserves full history
- **Queryable**: "Why did we choose X?" returns the decision, its reasoning, its dependencies
- **Live**: Changes propagate in real-time to all participants
- **Correctable**: Humans can fix extraction errors; corrections are events too

### Extraction, Not Documentation

Documentation fails because it requires humans to do extra work with no immediate benefit. The person writing docs already understands. The person who needs docs isn't there yet.

Extraction inverts this. Understanding is captured automatically from the conversations where it's created. The human and AI are discussing architecture. That discussion contains decisions, reasoning, alternatives. Extraction pulls structure from that conversation without requiring additional effort.

Extraction is imperfect. It misses things. It misinterprets things. But imperfect extraction is better than perfect documentation that doesn't exist. And extraction quality improves over time, both through better models and through human correction.

### The Duplex Flow

Duplex means two-way. In a duplex stream:

**Conversations flow in:**
Humans and AI discuss, debate, decide. Conversations are captured from configured sources.

**Understanding is extracted:**
Decisions, concepts, relationships are pulled from conversations. Each extraction is attributed and confidence-scored.

**Understanding flows back out:**
When starting new work, AI tools can query the understanding layer. "What decisions exist about authentication?" "What constraints apply to this module?" The query is explicit, not silent injection.

This creates a feedback loop. Conversations produce understanding. Understanding improves conversations. Better conversations produce better understanding.

### Query, Not Injection

Understanding flows back out through explicit query, not automatic context injection.

**Why not automatic injection:**
- Context overload: a mature codebase has hundreds of relevant decisions
- Relevance is task-dependent: the AI tool knows what it's doing
- Transparency: users should see what context is being used
- Control: users should be able to scope queries

**How it works:**
- AI tools have MCP access to query the understanding layer
- Starting a task, the tool can ask: "What decisions exist about [topic]?"
- Results return summaries ranked by relevance, recency, status, confidence
- Tool fetches full reasoning for decisions it deems relevant
- Human can also explicitly scope: "Consider only decisions from workspace X"

**Ranking factors:**
- Semantic relevance to query
- Recency (recent decisions weighted higher)
- Status (active > tentative > superseded)
- Confidence score from extraction
- Human verification (verified > unverified)

---

## The Platform

### Architecture

Duplex stream runs on Cloudflare's developer platform:

- **Workers**: API and application logic at the edge
- **Durable Objects**: Real-time workspace state with WebSocket connections
- **D1**: SQLite databases for persistent storage (one per org)
- **R2**: Object storage for large artifacts
- **Vectorize**: Semantic search over understanding
- **AI Gateway**: LLM calls with logging, caching, rate limiting
- **Workflows**: Durable multi-step processes (extraction pipeline)

This architecture enables:

- **Low latency**: Edge deployment means fast responses globally
- **Real-time collaboration**: Durable Objects maintain live connections
- **Transparent costs**: Cloudflare pricing is public and predictable
- **Horizontal scale**: No single points of failure

### Data Model

```
Organization
└── Workspace (many)
    ├── Participants (many)
    ├── Concepts (many)
    │   └── Relationships (many)
    ├── Decisions (many)
    │   ├── Appearances (many)
    │   ├── Alternatives (many)
    │   └── Dependencies (many)
    ├── Conversations (many)
    │   └── Messages (many)
    └── Integrations (many)
```

**Organization** is the billing and administration boundary. One D1 database per org.

**Workspace** is where understanding lives. Teams can have multiple workspaces for different domains. Workspaces can be queried independently or across the org.

**Configuration cascades**: Org → Workspace → Project → Individual. Settings at higher levels provide defaults. Lower levels can override.

### Integrations

Duplex stream connects to where work happens:

**Capture** (conversations flow in):
- Claude Code: File watching on ~/.claude/projects/
- Claude Web: Manual export, future API
- Cursor, Windsurf, other AI tools: Adapters for each format
- GitHub: PR discussions, issue threads
- Linear: Issue discussions
- Slack: Channel conversations (with consent, see Privacy)

**Query** (understanding flows out):
- MCP Server: AI tools can query the understanding layer
- API: Programmatic access for any integration
- CLI: Command-line queries and triggers

**Action** (understanding produces artifacts):
- GitHub: Generate ADRs, update READMEs
- Linear: Create issues from decisions
- Docs: Export to documentation systems

### Extraction Pipeline

The extraction pipeline is a Cloudflare Workflow with durable steps:

1. **Parse**: Convert source format (JSONL, JSON, etc.) to normalized messages
2. **Identify**: LLM scans full conversation, finds decisions and their locations
3. **Extract**: For each decision, LLM extracts reasoning, alternatives, dependencies
4. **Store**: Write to D1 with full relationship graph

Each step is durable. If extraction fails on decision 5 of 12, it resumes from decision 5, not from the beginning.

Extraction uses Claude Sonnet 4.5 through AI Gateway. Two-phase approach keeps context manageable for long conversations.

**On AI thinking blocks:**
Extended thinking is included in extraction input. Thinking blocks often contain deliberation that doesn't appear in final responses: "Option A has this problem, option B has that problem..." This is closer to reasoning-in-progress than the polished output. Whether it's "real" reasoning is philosophically unclear, but it's more revealing than responses alone.

### Human Correction

Extraction is imperfect. Humans can correct errors:

**Correction workflow:**
- Each extracted decision has edit controls
- Human can modify: title, summary, reasoning, status, dependencies, alternatives
- Changes are events (original preserved, correction layered on top)
- Corrections are attributed: "Corrected by [participant] on [date]"

**Learning from corrections:**
- Corrections become few-shot examples in future extraction prompts
- Aggregate patterns inform prompt engineering ("users often correct dependency relationships")
- No fine-tuning (too heavy); prompt evolution based on correction patterns

**Trust signals:**
- Human-verified decisions can be flagged as such
- Search results can weight verified higher
- Confidence scores reflect extraction confidence, not truth

### Configuration Hierarchy

Settings cascade from org to individual:

```
Organization (billing, compliance, defaults)
└── Workspace (team settings, integrations)
    └── Project (repo-specific config)
        └── Individual (personal preferences)
```

Examples:
- Org sets approved LLM providers
- Workspace enables specific integrations
- Project configures extraction triggers
- Individual sets notification preferences

Lower levels inherit from higher levels. Override only what's different.

---

## Privacy and Consent

### The Model

Capture requires consent. The default is explicit opt-in for personal sources, opt-out available for work sources.

**Control levels:**

| Level | Who controls | Examples |
|-------|--------------|----------|
| Org policy | Org admins | "Never capture Slack DMs", "Require opt-in for all sources" |
| Workspace | Workspace admins | "This workspace captures from these sources" |
| Conversation | Individual | "Don't capture this conversation" |
| Segment | Individual | "Off the record" markers within a conversation |

### Source-Specific Defaults

**High-consent sources** (default: opt-in required):
- Slack DMs
- Email
- Personal notes

**Work-context sources** (default: capture enabled, opt-out available):
- Claude Code sessions in work projects
- PR discussions
- Issue threads

### Ephemerality Controls

Not everything should be captured. Mechanisms for keeping things off the record:

- **Conversation-level**: Mark entire conversation as "don't capture"
- **Segment-level**: Mark messages as "off the record" / "resume capture"
- **Time delay**: Conversations aren't captured for N minutes, allowing deletion
- **Incognito mode**: If the AI tool has an incognito mode, respect it

### What's Visible to Whom

- Participants see conversations they participated in
- Workspace members see extracted decisions (not full conversations unless permitted)
- Org admins see aggregate metrics, not content
- No one outside org sees anything (unless explicitly shared)

Transparency about access: users can see who has access to what.

---

## Cold Start and Bootstrap

### The Problem

The value proposition requires accumulated understanding. Day one, the understanding layer is empty. Nothing to query. Nothing to flow back into conversations.

### Immediate Value (Before Critical Mass)

Even empty, duplex stream provides value:

**Single conversation extraction:**
"I can find that decision from last week" is a win, even with only one conversation extracted.

**Structure creation:**
The act of extraction creates queryable structure. Even a few decisions are more findable than scattered Slack messages.

**First successful query:**
The moment someone asks "why did we do X?" and gets an answer, the value is demonstrated.

### Bootstrap Mechanisms

**Retroactive extraction:**
- Claude Code: Extract from existing ~/.claude/ history
- Git: Extract from commit messages, PR descriptions, existing ADRs
- Docs: Import existing documentation (even stale) to seed concepts

**Onboarding conversations:**
When new team members get context from existing members, those conversations are rich with institutional knowledge. Capture and extract them.

**Accelerated seeding:**
For teams starting fresh, a "seed workspace" flow:
- Import existing docs and code
- Run extraction to identify concepts and implicit decisions
- Human review to verify/correct
- Baseline understanding in hours, not months

### The Honest Framing

Cold start isn't solvable. It's manageable.

Value compounds over time. Early adopters invest in future leverage. Like CI/CD or type systems: overhead at first, essential later.

The question is whether the value curve rises fast enough that early users stick around. Single conversation extraction provides immediate, tangible value. That buys time for understanding to accumulate.

---

## The Product

### Understanding Stream

The primary interface is the understanding stream: a real-time feed of how understanding is evolving.

New decision extracted. Concept definition updated. Relationship added between two ideas. Human corrected an extraction. AI elaborated on reasoning.

The stream shows activity, but activity that produces understanding. Not "John sent a message" but "A decision was made about authentication architecture."

### Decision Explorer

Query decisions and their relationships:

- Search by keyword, concept, participant, time
- View decision with full reasoning and alternatives
- See dependency graph: what this decision depends on, what depends on it
- Trace back to source: the original conversation, the specific messages
- View temporal context: what definitions applied when this decision was made

"Why did we choose file watching over MCP?" returns the decision, its reasoning, the alternatives considered, and links to the conversation where it was discussed.

### Concept Map

Visual representation of how concepts relate:

- Concepts as nodes, relationships as edges
- Filter by type, status, participant, time range
- Zoom from high-level architecture to implementation details
- See gaps: concepts referenced but never defined
- Time travel: view the map as it existed at any point

### Workspace Configuration

Configure how the workspace operates:

- Which integrations are enabled
- What triggers extraction (automatic, manual, scheduled)
- Who can access, with what permissions
- Privacy settings: what sources are captured, what consent is required
- Cost visibility: what this workspace has consumed

### Cost Transparency

Every workspace shows its actual infrastructure costs:

- Storage: D1 rows, R2 objects
- Compute: Worker invocations, Durable Object duration
- AI: LLM tokens consumed through AI Gateway
- Network: Bandwidth, WebSocket connections

Users see what they consume. No hidden subsidies. No attention-economy pricing where users are the product.

---

## The Economics

### Transparent Infrastructure Pricing

Duplex stream charges actual infrastructure costs plus a margin.

Traditional SaaS hides costs behind per-seat pricing. You don't know what you're paying for. You can't optimize. The vendor captures all efficiency gains.

Transparent pricing inverts this:

- You see exactly what each operation costs
- You can optimize your usage patterns
- Efficiency gains flow to you, not just the vendor
- Pricing scales predictably with usage

This requires infrastructure with public, predictable pricing. Cloudflare's developer platform provides this.

### Credit-Based Consumption

Users purchase credits. Operations consume credits at transparent rates.

| Operation | Cost |
|-----------|------|
| Store 1,000 messages | X credits |
| Extract decisions from conversation | Y credits |
| Query understanding layer | Z credits |
| Real-time sync (per hour) | W credits |

Large operations show cost before execution. "This extraction will cost approximately 15 credits. Proceed?"

No surprise bills. No opaque "enterprise pricing." Anyone can calculate what their usage will cost.

### The Margin

Duplex stream is a business, not a charity. Infrastructure cost plus margin equals price.

The margin pays for:
- Development and maintenance
- Support and documentation
- Infrastructure redundancy
- The team building this

The margin is visible. Users know what they're paying for infrastructure versus what they're paying for the service layer on top.

---

## The Business

### Commercial Open Source

Duplex stream is open source under AGPL.

AGPL means:
- Anyone can run it themselves
- Modifications must be shared under the same license
- Network use counts as distribution (no SaaS loophole)

This creates a natural boundary:
- Self-hosters get full functionality, handle their own ops
- Hosted service provides the managed experience
- Both benefit from the same codebase

### Why AGPL Works

AGPL protects against "strip-mining": taking open source, building proprietary services on top, contributing nothing back.

Companies that can run their own infrastructure can self-host. They get the software for free. If they improve it, those improvements flow back to everyone.

Companies that want managed service pay for it. They get operational reliability, support, and integration. The revenue funds continued development.

### Moat Analysis

Honest assessment of competitive defensibility:

**Not a moat** (others can replicate):
- Cloudflare architecture (friction, not barrier)
- Feature set (features are copyable)
- UI/UX (can be imitated)

**Emerging moat** (builds over time):
- Understanding layer data (valuable only after usage accumulates)
- Network effects (more users → more integrations → more users)
- Extraction quality (improves with more training data from corrections)

**True differentiation**:
- Transparent economics (ideological, most won't copy)
- Paradigm ownership (if "duplex stream" becomes the category name)
- Community (if contributors and users form around the project)

Early stage has no moat. Just execution speed and resonance before someone with distribution notices.

---

## The Name

### Duplex

Duplex means two-way, simultaneous flow. Like a duplex communication channel where both parties can transmit at once.

In duplex stream:
- Humans and AI both contribute
- Conversations flow in, understanding flows out
- Capture and query happen continuously

Not simplex (one-way). Not half-duplex (taking turns). Full duplex: continuous bidirectional flow of understanding.

### Stream

Stream means continuous flow, not batch. Real-time, not periodic.

Understanding isn't a snapshot. It's a stream of events. Decisions made, concepts refined, relationships discovered. The stream is always flowing.

Stream also evokes "stream of consciousness." The understanding layer is the persistent form of shared cognition.

---

## Who It's For

### Primary: Teams with AI-Augmented Development

Teams where humans and AI work together on complex systems.

They're already using Claude Code, Cursor, Copilot. They're getting value from AI assistance. But they're also feeling the pain of lost context, unexplained decisions, code that no one understands.

They want the speed of AI assistance without the comprehension debt.

### Secondary: Organizations with Knowledge Management Pain

Enterprises where institutional knowledge is critical but poorly captured.

They've tried wikis, documentation systems, knowledge bases. These systems are always out of date. No one maintains them. Critical knowledge lives in people's heads, Slack threads, email chains.

They want understanding that captures itself, not documentation that requires constant maintenance.

### Not For (Yet)

- Solo developers (collaboration is the point)
- Teams not using AI (no conversation stream to capture)
- Organizations that can't adopt new tools (integration requirements)

---

## The Vision

### Near Term (6-12 months)

Prove extraction works. Ship basic product. Get teams using it.

- Extraction pipeline handles Claude Code, Claude Web, Cursor
- Decision search and exploration works reliably
- Real-time workspace sync is stable
- Transparent pricing model is validated
- Human correction workflow is usable

Success: Teams say "I can finally understand why this was built this way."

### Medium Term (1-2 years)

Become infrastructure for AI-assisted work.

- Extraction quality rivals human documentation
- Integration ecosystem covers major tools
- Understanding layer has semantic search, inference
- Protocol is standardized, other tools can implement
- Cold start is measured in hours, not months

Success: "Duplex stream" is a category, not just a product.

### Long Term (3-5 years)

Understanding layer becomes essential infrastructure.

- Every AI-assisted project has an understanding layer
- Decisions are queryable across organizations (with consent)
- Collective understanding emerges from aggregated insights
- AI assistants expect understanding layers to exist

Success: You wouldn't ship software without CI/CD. You wouldn't ship without an understanding layer either.

---

## Principles

### Capture Everything, Extract What Matters

Don't filter at the source. AI tools don't know what context will matter later. Capture complete conversations. Let extraction decide what's significant. Humans can correct and refine.

### Make the Implicit Explicit

Reasoning that stays in someone's head isn't shared understanding. Decisions without documented alternatives aren't verifiable. Dependencies that aren't mapped are surprises waiting to happen. Surface what's hidden.

### Optimize for Queryability

The understanding layer succeeds if people can find what they need. "Why did we decide X?" must have an answer. Structure, index, and connect everything to enable discovery.

### Earn Trust Through Transparency

No hidden costs. No opaque algorithms. No surveillance disguised as features. Show what's captured, what's extracted, what it costs, who can see it. Trust is built by being trustworthy.

### Build for Collaboration

Single-player tools don't create shared understanding. Every feature should consider: how does this work with multiple participants? How do humans and AI interact with this? How does this scale to teams?

### Acknowledge Uncertainty

Extraction is imperfect. Reasoning is narrative. Confidence scores are estimates. Don't pretend false certainty. Surface what's known, what's inferred, and what's uncertain. Let humans judge credibility.

---

## Summary

Duplex stream is infrastructure for shared understanding between humans and AI.

**Paradigm**: Shared cognition over AI replacement. Understanding over activity. Transparency as foundation. Stated reasoning honestly framed.

**Protocol**: Concepts, relationships, decisions, participants, events. Temporally versioned. The understanding layer is extracted, connected, queryable, live, and correctable.

**Platform**: Cloudflare edge architecture. Durable extraction pipeline. Real-time workspace sync. Integrations for capture, query, and action. Human correction workflow.

**Product**: Understanding stream, decision explorer, concept map, cost transparency. Tools for building and querying shared understanding.

**Privacy**: Consent-based capture. Control at org, workspace, conversation, and segment levels. Ephemerality options. Transparency about access.

**Economics**: Transparent infrastructure pricing. Credit-based consumption. Visible margin.

**Business**: AGPL open source. Hosted service for managed experience. Community development.

One sentence: Duplex stream makes the reasoning behind software visible, persistent, and queryable, so humans and AI can build understanding together.