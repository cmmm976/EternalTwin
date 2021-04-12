import { CaseStyle } from "kryo";
import { $Any } from "kryo/lib/any";
import { $Date } from "kryo/lib/date";
import { $Float64 } from "kryo/lib/float64";
import { $Uint32 } from "kryo/lib/integer";
import { RecordIoType, RecordType } from "kryo/lib/record";
import { $Ucs2String } from "kryo/lib/ucs2-string";

import { $JobId } from "./job-id";
import { ShortStoredTask } from "./short-stored-task";
import { StoredTaskState } from "./stored-task-state";
import { $TaskId } from "./task-id";
import { $TaskStatus } from "./task-status";

export type StoredTask = ShortStoredTask & StoredTaskState;

export const $StoredTask: RecordIoType<StoredTask> = new RecordType<StoredTask>({
  properties: {
    id: {type: $TaskId},
    jobId: {type: $JobId},
    parent: {type: $TaskId, optional: true},
    status: {type: $TaskStatus},
    statusMessage: {type: $Ucs2String, optional: true},
    createdAt: {type: $Date},
    advancedAt: {type: $Date},
    stepCount: {type: $Uint32},
    runningTime: {type: $Float64},
    kind: {type: $Ucs2String},
    dataVersion: {type: $Uint32},
    options: {type: $Any},
    state: {type: $Any},
  },
  changeCase: CaseStyle.SnakeCase,
});
