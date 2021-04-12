import { CaseStyle } from "kryo";
import { $Date } from "kryo/lib/date";
import { RecordIoType, RecordType } from "kryo/lib/record";

import { $JobId, JobId } from "./job-id";
import { $TaskId, TaskId } from "./task-id";

export interface StoredJob {
  id: JobId,
  createdAt: Date,
  rootTask: TaskId,
}

export const $StoredJob: RecordIoType<StoredJob> = new RecordType<StoredJob>({
  properties: {
    id: {type: $JobId},
    createdAt: {type: $Date},
    rootTask: {type: $TaskId},
  },
  changeCase: CaseStyle.SnakeCase,
});

