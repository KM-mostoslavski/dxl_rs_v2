# Learnings

_One line per step ended, appended by the agent at step end. What to do differently
next time, or "nothing new"._

| date | module | step | learning |
|---|---|---|---|
| 2026-11-05 | bus | 5 | Run the CI feature-test legs (`std,<feature>`, `-D warnings`) before step end: moving a test between family modules leaves unused imports that only those legs catch, and they were already red on `dev`. |
| 2026-11-09 | bus | 5 | Before building a plan's setters, check that every pub type their signatures take can be built by a caller: `Voltage`, `Temperature` and `Load` had no constructor, so the typed limits needed core API the plan did not list. |
| 2026-11-09 | bus | 5 | A torque state the bus holds is a belief, not the hardware: Setup never reads Torque Enable, a lost reply leaves `Off`, and an AX that faults while on keeps its torque. A guard that permits (an EEPROM write) needs a confirmed state, set only by a reply that shows the instruction ran; a guard that refuses (a goal) can trust the belief. |
| 2026-11-10 | bus | 5 | Check every sibling branch for unmerged work before planning: `feature/bus` already held the typed limits (T096 to T103) that a fresh FR-10 plan would have rebuilt on `dev`. And a typed setter is only as wide as its table data: 70 of 76 tables had no `unit_source`, so the limits returned `NoUnit` on almost every model until the e-Manual pass (T104). |
