# EduTalent Dashboard UI/UX Remake — Engineering Implementation Plan

## 1. Purpose

This document is the implementation contract for a complete redesign of EduTalent's authenticated product dashboards before the UI-exploration defects are fixed in their own follow-up PRs.

The goal is not a cosmetic reskin. The goal is to replace the current visually noisy, glassmorphism-heavy, card-everywhere dashboard system with a calm, task-oriented, accessible, responsive, bilingual education operations product suitable for real school use.

The redesign must preserve working product behavior and security boundaries while improving information architecture, hierarchy, interaction clarity, accessibility, responsiveness, RTL/LTR quality, and consistency across every role:

- Platform Administrator;
- School Manager;
- Teacher;
- Student;
- Parent.

This plan is intentionally implementation-specific to the current Dioxus + Tailwind EduTalent codebase.

---

## 2. Non-negotiable product principles

### 2.1 Task first, decoration second

Every screen must make the user's next meaningful action obvious. Decorative gradients, blur layers, animated background blobs, excessive translucency, hover scaling, and ornamental widgets must not compete with school workflows.

The authenticated product should feel like a dependable enterprise tool, not a landing page.

### 2.2 Truthful UI only

A control must be one of:

1. functional;
2. deliberately disabled with an explicit explanation;
3. absent.

Do not render controls that look interactive but have no implemented behavior. Existing examples include the global search affordance and some utility actions.

Synthetic or unsupported metrics must not be invented to make dashboards appear richer. Real persisted or API-derived data is preferred; otherwise use useful actions, recent work, empty states, or explanatory content.

### 2.3 One shared product system, role-specific priorities

All roles use the same shell, primitives, spacing, typography, feedback model, form language, and responsive rules. Role dashboards differ by information hierarchy and tasks, not by unrelated visual systems.

### 2.4 Accessibility is part of component design

Target WCAG 2.2 AA for product interactions and use WAI-ARIA Authoring Practices for dialogs, menu buttons, notifications, disclosures, and keyboard behavior.

At minimum:

- visible keyboard focus on every interactive element;
- minimum 24 x 24 CSS-pixel pointer targets, with 40–44px preferred for primary controls;
- no information conveyed by color alone;
- semantic headings and landmarks;
- logical keyboard order;
- Escape-close and focus restoration for modal/dialog surfaces;
- no focus hidden behind sticky navigation;
- reduced-motion support;
- accessible loading, success, warning, error, empty, and disabled states;
- labels programmatically associated with form controls;
- screen-reader names for icon-only buttons.

### 2.5 RTL and LTR are first-class layouts

Persian cannot be treated as a translated LTR layout.

Use logical CSS properties and direction-aware component anatomy. Avoid hardcoded left/right assumptions except for inherently directional values.

Technical values that should remain LTR even in Persian UI must be isolated appropriately, including:

- email addresses;
- UUIDs;
- URLs;
- hashes;
- timestamps where formatting requires it;
- codes and identifiers.

Vazirmatn remains the primary Persian UI font. Poppins remains the Latin UI font unless a later branding decision changes it.

---

## 3. Current design problems to remove

The existing shared dashboard implementation has several systemic issues that the remake must eliminate.

### 3.1 Excessive glassmorphism and visual noise

Current pages rely on:

- translucent backgrounds;
- large blurred animated blobs;
- repeated `glass-card` surfaces;
- backdrop blur on navigation, cards, and menus;
- gradient avatars and brand tiles;
- scale-on-hover behavior on generic content cards.

This reduces hierarchy and makes operational screens feel visually unstable.

### 3.2 Card-everything layout

Lists, metrics, actions, content sections, status rows, forms, and loading states are frequently represented using the same floating rounded card. Important actions and passive information therefore look equally prominent.

The remake must use the right structure for the job:

- tables for tabular data;
- lists for repeated entities;
- panels for grouped tasks;
- compact stat blocks for real metrics;
- drawers/dialogs for focused edits;
- banners for alerts;
- empty-state regions for absent data;
- only use elevated cards when a card is semantically useful.

### 3.3 Weak information hierarchy

Current role overviews often present equal-weight action cards with limited context. The user must infer what to do next.

Each role overview must instead answer:

1. What needs my attention now?
2. What changed recently?
3. What are my primary workflows?
4. Where do I go next?

### 3.4 Inconsistent feedback

Different surfaces use unrelated inline text, raw server-function errors, silent `.ok()` handling, or no success feedback.

The new shared feedback system must define consistent:

- success notices;
- warning notices;
- error notices;
- retry actions;
- loading skeletons;
- empty states;
- optimistic/busy button behavior.

### 3.5 Responsive behavior is too binary

The current layout checks viewport width once and switches between separate desktop/mobile shells. This can become stale after resizing and creates two different product structures.

The remake should prefer one responsive shell whose CSS adapts naturally, with only narrowly scoped behavior state where JavaScript is actually required.

### 3.6 Navigation is visually heavy and behaviorally inconsistent

Current navigation includes a large branded sidebar, collapsible state, glass surfaces, bottom navigation, separate profile blocks, and a top header with several competing controls.

The remake must simplify navigation and make location obvious.

### 3.7 Utility controls compete with core work

The header currently contains search, notification, time, welcome text, sidebar control, and dashboard title. Some utilities are not fully functional.

The top bar should contain only controls that contribute to the current workflow.

---

## 4. Visual direction

### 4.1 Product character

The design direction is:

**calm enterprise education software**

Keywords:

- precise;
- trustworthy;
- warm but professional;
- low visual noise;
- bilingual;
- accessible;
- task-oriented;
- modern without fashion-driven effects.

### 4.2 Background and surfaces

Light mode:

- app background: `#F7F8FB`;
- primary surface: `#FFFFFF`;
- secondary/soft surface: `#F2F4F7`;
- strong border: `#D8DCE5`;
- soft border: `#E7E9EF`.

Dark mode:

- app background: approximately `#0F1117`;
- primary surface: approximately `#171A22`;
- secondary surface: approximately `#1E222C`;
- border: low-contrast neutral, not translucent white glass.

No ambient animated background blobs.

No full-screen pastel gradients.

Backdrop blur is allowed only where it materially improves a temporary overlay, never as the default surface language.

### 4.3 Brand accent

Keep violet as EduTalent's primary brand accent but use it with restraint.

Recommended hierarchy:

- primary accent: violet around `#6D5DFB`;
- hover/pressed: darker violet;
- soft selected background: very light violet tint;
- brand gradient: permitted only for the compact EduTalent mark if retained.

Do not use violet for every icon, border, heading, badge, and button.

### 4.4 Semantic colors

Use distinct semantic roles:

- success: green;
- warning: amber;
- danger/destructive: red;
- info: blue;
- neutral/inactive: gray.

Each state must also include text/icon/label meaning so color is redundant, not exclusive.

### 4.5 Shadows and borders

Default hierarchy should come from spacing, typography, background, and 1px borders.

Use shadows sparingly:

- no shadow on routine rows;
- subtle shadow on floating menus/dialogs;
- small elevation on selected/high-priority panels only.

### 4.6 Corner radius

Recommended system:

- controls: 8–10px;
- panels: 12–16px;
- dialogs: 16px;
- pills only for semantic chips/status values.

Avoid giant rounded containers around every section.

---

## 5. Typography system

### 5.1 Families

- Persian: Vazirmatn;
- Latin: Poppins with system fallbacks;
- technical identifiers: monospace where readability benefits.

### 5.2 Scale

Recommended product scale:

- page title: 28–32px desktop, 24–28px compact/mobile;
- section title: 18–20px;
- card/panel title: 15–16px;
- body: 14–16px;
- labels: 13–14px;
- supporting/caption text: 12–13px;
- table text: 13–14px.

Do not use 10px text for important navigation labels.

### 5.3 Weight

Use weight to establish hierarchy, not decoration:

- page title: 700;
- section title: 600–700;
- control labels: 500–600;
- body: 400;
- metadata: 400–500.

### 5.4 Line height

Persian text requires more vertical breathing room. Preserve comfortable line height without making compact controls excessively tall.

---

## 6. Spacing and sizing system

Use a consistent 4px-based rhythm:

- 4px micro spacing;
- 8px tight control spacing;
- 12px related-content spacing;
- 16px standard element spacing;
- 24px panel/section spacing;
- 32px major separation;
- 40–48px page-level separation where needed.

Interactive controls should normally be at least 40px high. Primary form controls should generally use 44px.

Desktop content width should remain readable on wide monitors rather than stretching endlessly. Use a centered content region with a practical max width around 1440px, while tables or dense workflows may intentionally use the available width.

---

## 7. New shared application shell

### 7.1 Desktop structure

Use three stable regions:

1. navigation rail/sidebar;
2. compact top bar;
3. scrollable content workspace.

Recommended desktop sidebar width:

- expanded: about 240–256px;
- optional compact state only if it materially improves smaller laptop layouts;
- collapsed state must preserve accessible names/tooltips and not become the default visual gimmick.

The sidebar should use a solid surface and border, not glassmorphism.

### 7.2 Sidebar anatomy

Top:

- compact EduTalent brand mark + product name;
- no oversized logo tile.

Middle:

- role-specific navigation;
- selected item shown with a soft violet background, strong text, and a restrained accent indicator;
- consistent 40–44px row height;
- icon + label;
- no hover scaling.

Bottom:

- user identity summary;
- language control;
- profile entry;
- sign out as a secondary/destructive utility.

Avoid nested decorative profile cards.

### 7.3 Top bar

The top bar must show:

- current section/page title;
- optional concise context/breadcrumb when useful;
- notification control if functional;
- mobile navigation trigger when needed.

Remove global search from the shell until a real product search capability exists. Do not ship inert search boxes or search buttons.

Time/date should not consume persistent top-bar space unless a role workflow needs it.

### 7.4 Content workspace

Use a predictable page frame:

- page header;
- optional primary action;
- page-level feedback region;
- content sections;
- task/detail surfaces.

The first viewport should show the most important work, not decorative padding.

---

## 8. Mobile and tablet architecture

### 8.1 One responsive system

Do not maintain unrelated desktop and mobile product designs.

Use the same component hierarchy and information architecture with responsive composition.

### 8.2 Mobile navigation

For roles with a small number of primary destinations, a bottom navigation bar is acceptable, but:

- maximum 4–5 primary items;
- remaining destinations must be available from a clear More/Menu surface;
- labels should not be 10px microtext;
- selected state should not depend solely on color;
- safe-area insets must be respected.

For roles with many destinations, use a menu/drawer rather than silently omitting later items.

### 8.3 Mobile header

Show:

- current page title;
- navigation/menu trigger where required;
- notification/profile utility only if it remains useful.

Do not duplicate large user/profile panels.

### 8.4 Responsive forms

- one column on narrow screens;
- two columns only when field relationships benefit;
- labels always visible;
- action bar remains reachable with on-screen keyboard;
- no horizontal scrolling for normal forms.

### 8.5 Responsive tables

Prefer one of:

- horizontal scroll with sticky key column and clear overflow affordance;
- stacked semantic rows for compact data;
- table-to-list adaptation when meaning is preserved.

Do not simply shrink text until the table fits.

---

## 9. Core component system

The remake should introduce or normalize reusable components rather than continuing one-off Tailwind strings.

### 9.1 Layout primitives

Create/standardize:

- `AppShell` / current dashboard layout;
- `PageHeader`;
- `PageSection`;
- `Surface` / panel;
- `Toolbar`;
- `SplitPane` for list/detail workflows where useful;
- `ActionGroup`.

### 9.2 Buttons

Variants:

- primary;
- secondary;
- quiet/ghost;
- destructive;
- icon-only;
- inline/link action.

Every button must define:

- hover;
- focus-visible;
- active;
- disabled;
- busy state.

Primary actions should be scarce. A screen should rarely have more than one dominant primary action in the same decision area.

### 9.3 Form fields

Standard field anatomy:

- visible label;
- required marker when actually required;
- optional helper text;
- control;
- validation/error text;
- disabled/read-only treatment.

The visual required marker, HTML/client validation, typed request validation, and server validation must agree.

### 9.4 Feedback

Shared feedback components:

- success banner/toast;
- info banner;
- warning banner;
- error banner;
- inline field error;
- retry block;
- loading skeleton;
- empty state.

Raw internal server-function text should not be the main user-facing message.

### 9.5 Status chips

Status chips must be concise and semantic.

Examples:

- Submitted;
- Needs review;
- Published;
- Archived;
- Pending grading;
- Overdue.

Status should never be represented only by a color dot.

### 9.6 Dialogs and destructive actions

Use accessible modal dialogs for high-impact operations.

Required behavior:

- explicit title;
- clear consequence;
- least-destructive action receives sensible initial focus where appropriate;
- Escape closes when safe;
- focus trapped while open;
- focus restored to invoker after close;
- destructive action visually distinct;
- busy state prevents duplicate submissions.

### 9.7 Tables/lists

Tables need:

- clear headers;
- consistent row density;
- empty state;
- loading state;
- row actions grouped consistently;
- no hover movement;
- visible keyboard focus for actionable rows/controls.

Lists should use dividers and spacing instead of wrapping every item in an elevated card.

---

## 10. Role dashboard information architecture

## 10.1 School Manager

Primary mental model: **operate the school**.

Overview should prioritize:

1. primary actions;
2. items needing attention;
3. recently changed operational data;
4. access to users/classes/knowledge/settings.

Recommended overview structure:

### Header

- `School overview`;
- concise explanation of scope;
- optional one primary action such as `Add user` only after the provisioning workflow is production-supported.

### Operational shortcuts

Use a compact horizontal/2x2 action region, not four giant cards.

Actions:

- User management;
- Class management;
- Knowledge submissions;
- Settings.

### Attention region

Only use real values. Examples if available:

- provisioning errors;
- classes without teachers;
- knowledge submissions awaiting next stage;
- other server-backed operational warnings.

If unavailable, omit the region.

### Recent/summary region

Use real lists or counts only.

Do not reintroduce synthetic activity, fake uptime, fake student counts, or invented trend percentages.

## 10.2 Teacher

Primary mental model: **teach, assign, review**.

Overview hierarchy:

1. pending grading;
2. upcoming/recent assignments;
3. classes/students;
4. knowledge assets.

Use the existing real teacher dashboard stats, but redesign them as compact stat blocks rather than generic glass cards.

Recommended layout:

- small summary strip for real `total_classes`, `total_students`, `pending_grading`;
- prominent `Needs attention` list driven by pending grading where feasible;
- recent assignments table/list;
- quick actions for assignments, grading, and knowledge assets.

## 10.3 Student

Primary mental model: **what do I need to do next?**

Overview hierarchy:

1. upcoming/pending/overdue assignments;
2. current classes;
3. grades where enabled/available.

Do not make enrolled-class count the dominant message if assignments are more actionable.

Recommended layout:

- `Up next` assignment list;
- overdue clearly but calmly highlighted;
- class list with teacher/term metadata;
- small summary values only where they are useful.

## 10.4 Parent

Primary mental model: **understand my children's school progress**.

Overview should center the child relationship rather than generic dashboard widgets.

Recommended layout:

- child selector/list if multiple children;
- child summary;
- recent grades/assignments/alerts when supported;
- reports/communication only when product capability flags enable them.

Disabled capabilities must not appear as fake placeholders.

## 10.5 Platform Administrator

Primary mental model: **govern platform-wide controlled workflows**.

The overall shell should visually align with every other role but support denser governance information.

Knowledge administration requires:

- clear asset state;
- clear school ownership;
- clear next allowed action;
- visible lifecycle history;
- explicit failure reason when present.

The known knowledge-lifecycle UX defects are tracked separately in the exploration plan and should be fixed in their own follow-up PR rather than silently folded into this visual-foundation PR.

---

## 11. Page-level patterns for existing feature screens

### 11.1 User management

Use:

- page header + create action;
- role filter/search only if functional;
- dense user list/table;
- secondary metadata aligned consistently;
- create/edit flows in a focused page section or dialog;
- success/error feedback near the action origin.

Known provisioning endpoint defects remain a separate follow-up PR.

### 11.2 Class management

Use:

- page header;
- class list/table or compact structured cards only when cards improve scanability;
- clear subject, term, teacher, student count;
- create action separated from passive content;
- predictable create/edit form.

Known class RLS recursion and create-success feedback defect remain a separate follow-up PR.

### 11.3 Assignments

Teacher:

- assignment table/list with due date, class, submission progress, status;
- one clear create action;
- draft/published/closed status treatment.

Student:

- urgency-first sort/filter;
- pending/overdue/completed distinctions;
- assignment details and submit action clearly separated.

### 11.4 Grading/submissions

Use a review-workflow layout, not a generic card grid:

- submission queue/list;
- selected submission detail;
- grading controls;
- save/next actions;
- clear persisted success feedback.

### 11.5 Knowledge submissions

The visual redesign should clarify the manager workflow while leaving functional upload expansion for its dedicated issue PR.

Presentation should clearly distinguish:

- source registration;
- registered/submitted state;
- downstream OCR/embed/publish stages.

### 11.6 Settings/profile

Use grouped settings sections with consistent labels and descriptions. Avoid nesting cards inside cards.

---

## 12. Interaction and motion

Motion should explain state, not decorate the dashboard.

Allowed examples:

- menu/dialog fade/scale around 120–180ms;
- row insertion/update transition;
- subtle sidebar/drawer transition;
- loading skeleton shimmer only if reduced motion is respected.

Remove:

- animated ambient blobs;
- pulse animation on decorative elements;
- universal hover scale on content panels;
- large transform animations on routine navigation.

Respect `prefers-reduced-motion: reduce` and provide effectively static alternatives.

---

## 13. Loading, empty, error, and offline/dependency states

Every server-backed section must define all four states.

### Loading

Use geometry-preserving skeletons or concise loading regions. Avoid whole-page spinners when existing shell/content can remain visible.

### Empty

Explain:

- what is empty;
- whether that is normal;
- what the user can do next, if anything.

### Error

User-facing errors should be safe and actionable.

Pattern:

- plain-language problem;
- retry action where valid;
- no SQL, token, internal URL, stack trace, or raw `ServerFnError` as primary copy.

### Partial success

Distinguish durable mutation success from refresh/display failure.

Example:

`Class created, but the class list could not be refreshed.`

This rule is especially important for the exploration issues already discovered.

---

## 14. Bilingual content quality

The redesign must not mix English and Persian casually within one localized workflow.

Requirements:

- move new visible strings into localization resources when the surrounding page is localized;
- use consistent Persian terminology;
- avoid machine-like literal translations;
- keep technical terms readable and directionally isolated;
- verify long Persian labels do not overflow navigation, tables, buttons, and dialogs;
- verify English mode separately.

Platform governance terminology that remains English must be an explicit product decision, not an accidental hardcoded string.

---

## 15. Accessibility implementation checklist

For this remake PR:

- [ ] page landmarks are meaningful;
- [ ] one logical page heading per active view;
- [ ] sidebar/nav exposes the selected/current item semantically;
- [ ] all icon-only buttons have accessible names;
- [ ] visible focus treatment is at least clearly equivalent to a strong 2px indicator;
- [ ] keyboard focus is never obscured by sticky shell elements;
- [ ] menus/dialogs follow expected keyboard behavior;
- [ ] mobile menu is keyboard and screen-reader operable;
- [ ] target sizes meet WCAG 2.2 minimums;
- [ ] body and control text contrast is sufficient;
- [ ] non-text controls/borders/focus indicators have sufficient contrast;
- [ ] status meaning is redundant with text/icon, not color alone;
- [ ] reduced-motion mode disables unnecessary motion;
- [ ] form errors are connected to the relevant fields where feasible;
- [ ] RTL focus/order behavior is coherent;
- [ ] desktop and mobile zoom/reflow do not clip primary content.

Human accessibility acceptance remains part of the separate manual/external production acceptance track and is not replaced by automated checks.

---

## 16. Code architecture plan

### 16.1 Shared styling source of truth

Refactor `packages/web/input.css` and Tailwind theme tokens so the source stylesheet—not generated `assets/main.css`—defines the visual system.

Then rebuild `assets/main.css` using the existing `pnpm build:css` script.

### 16.2 Shared components to refactor first

Primary files:

- `packages/web/src/views/role_based/components/dashboard_layout.rs`;
- `packages/web/src/views/role_based/components/sidebar.rs`;
- `packages/web/src/views/role_based/components/header.rs`;
- shared component exports;
- `packages/web/src/application/routing_service.rs` only where navigation presentation requires truthfulness changes.

### 16.3 Shared primitives

Prefer small reusable Dioxus components for recurring patterns such as:

- page headers;
- stat blocks;
- section headers;
- feedback banners;
- status labels;
- empty/loading/error states;
- action buttons.

Do not create a monolithic dashboard component.

### 16.4 Role overview refactors

Refactor the overview components for:

- School Manager;
- Teacher;
- Student;
- Parent;
- Platform Admin where applicable.

Reuse real existing API resources. Do not change authorization or persistence behavior in this PR.

### 16.5 Existing feature pages

Use the new global primitives/tokens so existing feature pages immediately inherit the calmer visual system. Apply targeted markup changes only where necessary to prevent obvious nested-card, spacing, table, form, or responsiveness problems.

Functional changes tied to UI-exploration issues remain isolated to their later PRs.

---

## 17. Explicit scope boundary for the first redesign PR

### Included

- dashboard visual design system;
- shared shell redesign;
- sidebar/navigation presentation;
- top-bar redesign;
- responsive/mobile shell redesign;
- typography, color, spacing, elevation, and motion system;
- shared form/table/feedback styling improvements;
- role overview information hierarchy;
- removal/hiding of inert global shell controls;
- accessibility improvements directly related to the shared shell and visual primitives;
- RTL/LTR layout cleanup;
- regenerated compiled CSS;
- focused product-truthfulness and UI regression tests where practical.

### Excluded and reserved for follow-up issue PRs

The redesign PR must not silently mix in the already recorded backend/workflow fixes:

1. enrollment/students RLS recursion in class-list reads;
2. disabled School Manager Student/Teacher/Parent provisioning endpoint and provisioning safety redesign;
3. direct local-file knowledge upload capability;
4. Platform Admin knowledge lifecycle action gating / archive confirmation / lifecycle-specific workflow semantics.

The redesign may prepare shared UI primitives that those later PRs reuse, but it must not change their authorization, database, provisioning, storage, or lifecycle semantics.

---

## 18. Implementation order for PR 1

1. Add this design/engineering contract.
2. Refactor Tailwind theme tokens and source CSS.
3. Replace glassmorphism-based shared shell.
4. Refactor sidebar.
5. Refactor top bar and remove inert shell search.
6. Replace binary desktop/mobile layout with responsive shared structure where feasible.
7. Add/refine shared UI primitives.
8. Redesign School Manager overview.
9. Redesign Teacher overview.
10. Redesign Student overview.
11. Redesign Parent overview.
12. Visually normalize Platform Admin overview/governance shell without changing lifecycle semantics.
13. Normalize common forms, lists, tables, loading, and feedback appearance.
14. Rebuild generated CSS.
15. Run focused tests/checks.
16. Open PR, review diff, and run only relevant workflows.
17. Fix failures/review blockers until green and merge-ready.
18. Merge before starting the first exploration-fix PR.

---

## 19. Focused verification for PR 1

### Required local/repository checks

At minimum:

- Tailwind CSS rebuild succeeds;
- generated `assets/main.css` matches source CSS;
- Rust formatting/check for touched web code;
- `cargo check`/relevant web compile for the production server feature set;
- web/unit tests relevant to routing/product truthfulness/shared components;
- no endpoint authorization manifest or migration changes unless unexpectedly required and separately justified.

### Required CI/workflows

Run only workflows that materially cover the changed surfaces. Avoid unrelated expensive appliance/AI/package workflows when the diff is frontend-only and repository policy does not require them.

Likely gates:

- frontend/web build/test check;
- code quality/lint/format check;
- product-truthfulness test if separately gated;
- any repository-required PR gate that always runs.

The actual workflow list must be derived from the repository's current GitHub Actions configuration before declaring merge-ready.

### Visual QA

Because automated compilation cannot prove UI quality, manually verify at least:

- desktop 1440px-class viewport;
- compact laptop/tablet width;
- mobile approximately 390–430px width;
- Persian RTL;
- English LTR;
- light mode;
- dark mode if supported by the active product;
- keyboard navigation through sidebar/top bar/core overview actions;
- notification menu;
- long labels and empty/error/loading states.

No primary content may be clipped or hidden under navigation.

---

## 20. PR sequence after the dashboard remake

After PR 1 is green, review-clean, merge-ready, and merged, continue one issue at a time from `UI-Exploration-Fix-Plan.md`.

### PR 2 — Class list RLS recursion + class create feedback

Implement Issue 1 only:

- new forward migration;
- bounded security-definer relationship helper;
- RLS regression matrix;
- class-list error handling;
- truthful create success/partial-success feedback.

Run only DB/RLS/API/web checks required for that scope, make green, merge.

### PR 3 — School Manager user provisioning

Implement Issue 2 only:

- authoritative production provisioning workflow;
- Student/Teacher/Parent support;
- safe Auth/database compensation strategy;
- required-field contract cleanup;
- same-school relationship validation;
- endpoint authorization update only after hardened implementation;
- provisioning regression tests.

Run only relevant auth/API/RLS/web/security checks, make green, merge.

### PR 4 — Knowledge local file upload

Implement Issue 3 only:

- browser/local file intake;
- controlled internal storage;
- size/type/hash/path validation;
- tenant isolation;
- URL upload remains supported as an alternative;
- registration lifecycle remains truthful.

Run storage/security/API/web checks, make green, merge.

### PR 5 — Platform Admin governed knowledge lifecycle UX

Implement Issue 4 only:

- state-aware action gating;
- lifecycle stepper/next-action guidance;
- archive confirmation and consequence copy;
- human-readable audit trail;
- retry/failure guidance;
- frontend/backend state-machine regression alignment.

Run knowledge lifecycle/API/web tests, make green, merge.

If implementation proves that two issue PRs cannot be safely separated without duplicated or transitional architecture, stop and explicitly document the dependency before combining them. Do not combine scopes merely for convenience.

---

## 21. Definition of done for the dashboard remake

PR 1 is complete only when all of the following are true:

- [ ] animated ambient dashboard blobs removed;
- [ ] generic glassmorphism is no longer the primary surface model;
- [ ] generic cards no longer scale on hover;
- [ ] shared dashboard shell uses a calm solid-surface system;
- [ ] active navigation is immediately obvious in both RTL and LTR;
- [ ] inert global search UI is removed/hidden until functional;
- [ ] notification access remains functional and accessible;
- [ ] user/profile/language/sign-out utilities are consolidated and understandable;
- [ ] role overview pages have task-oriented information hierarchy;
- [ ] no fake metrics are introduced;
- [ ] shared form controls have coherent focus/label/error styling;
- [ ] common table/list surfaces are readable and responsive;
- [ ] loading/empty/error/success visuals are standardized;
- [ ] Persian RTL and English LTR both render without obvious layout defects;
- [ ] mobile navigation exposes all required destinations rather than silently dropping items;
- [ ] focus-visible and target-size accessibility requirements are met by shared controls;
- [ ] reduced-motion rules exist for nonessential animation;
- [ ] generated CSS is rebuilt and committed;
- [ ] relevant web checks/tests are green;
- [ ] required PR workflows are green;
- [ ] review contains no unresolved blocker;
- [ ] PR is mergeable and merge-ready;
- [ ] the PR does not claim to fix the separately tracked exploration defects unless explicitly moved into scope with documented rationale.

---

## 22. Final design quality gate

Before merge, evaluate the dashboard as a coherent product rather than isolated components.

Reject the remake if any of the following remain:

- prototype-looking glass panels;
- excessive gradients or blur;
- random card grids;
- weak page hierarchy;
- tiny navigation labels;
- mixed button styles;
- raw internal errors as normal UX;
- fake/inert controls;
- layout clipping in Persian RTL;
- desktop-only interaction assumptions;
- destructive actions visually equivalent to routine actions;
- hover movement on passive content;
- hidden mobile destinations;
- inconsistent spacing/typography between roles;
- accessibility dependent on pointer hover or color alone.

The finished dashboard should feel like one intentionally designed education operations platform, regardless of which role is signed in.
