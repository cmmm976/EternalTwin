import { TsEnumType } from "kryo/lib/ts-enum";

export enum TaskStatus {
  Running,
  Complete,
  Failed,
  Stopped,
}

export const $TaskStatus: TsEnumType<TaskStatus> = new TsEnumType<TaskStatus>({
  enum: TaskStatus,
});
