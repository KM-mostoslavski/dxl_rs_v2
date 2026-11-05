# Deferred

_Written by agents when a task hits a contract break (see
`CLAUDE.md`), and by me at step 9. Each entry re-enters the cycle at
step 4. Three open entries on one module = `needs contract rework`.
Never delete a row; set status `done` with the date._

| date | module | step | what was needed | why a contract break | smallest unblocking change | status |
|---|---|---|---|---|---|---|
- [feature] D-01 Setup (declare and autoscan) holds an actuator whose reply carries only a hardware fault flag (e.g. P1 Input Voltage) as Faulted instead of failing the whole bus, and returns those faults beside the held list, so the caller can still fix its EEPROM limits with write_raw after Setup. Today kmr's Ids::voltage_limits never runs on a servo already outside its voltage limits: Setup fails first. The tests fault_at_setup_fails_setup and p1_model_read_fault_fails_setup in src/bus/setup.rs pin the current behaviour. Needed by kmr docs/deferred.md row B2. (found during none step -)
