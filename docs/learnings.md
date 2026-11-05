# Learnings

_One line per step ended, appended by the agent at step end. What to do differently
next time, or "nothing new"._

| date | module | step | learning |
|---|---|---|---|
| 2026-11-05 | bus | 5 | Run the CI feature-test legs (`std,<feature>`, `-D warnings`) before step end: moving a test between family modules leaves unused imports that only those legs catch, and they were already red on `dev`. |
