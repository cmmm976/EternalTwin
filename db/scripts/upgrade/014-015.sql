COMMENT ON SCHEMA public IS '{"version": 15}';

CREATE DOMAIN etwin_task_status AS VARCHAR(8) CHECK (value IN ('Running', 'Complete', 'Failed', 'Stopped'));
CREATE DOMAIN etwin_task_id AS UUID;

-- The list of long-running tasks.
CREATE TABLE etwin_tasks (
  -- The (unique) task ID
  etwin_task_id etwin_task_id NOT NULL,
  -- Task creation time
  ctime INSTANT NOT NULL,
  -- Task last access time
  atime INSTANT NOT NULL,
  -- Parent task; it cannot run until the current task completes
  parent_task_id etwin_task_id,
  -- Status of the task
  status etwin_task_status NOT NULL,
  -- Status message of the task, for exemple if it failed.
  status_message TEXT,
  -- Number of steps executed
  step_count u32 NOT NULL,
  -- Total running time
  running_time INTERVAL NOT NULL,
  -- Name of the task kind (immutable)
  kind VARCHAR(64) NOT NULL,
  -- A version number for the data stored in `options` and `state`
  data_version u32,
  -- Options of the task (immutable)
  options JSONB NOT NULL,
  -- Internal state of the task
  state JSONB NOT NULL,

  -- Number of children preventing the execution of the task (i.e. with status != Complete)
  -- Must be manually maintained when updating tasks relationships
  _blocking_children u32 NOT NULL,


  PRIMARY KEY (etwin_task_id),
  CHECK (atime >= ctime),
  CONSTRAINT etwin_task__parent_task_id__fk FOREIGN KEY (parent_task_id) REFERENCES etwin_tasks(etwin_task_id) ON DELETE RESTRICT ON UPDATE CASCADE
);

CREATE INDEX etwin_tasks__atime_running__idx ON etwin_tasks(atime) WHERE status = 'Running';
