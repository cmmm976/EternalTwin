import { AuthContext } from "../auth/auth-context";
import { JobId } from "./job-id";
import { StoredJob } from "./stored-job";
import { StoredTask } from "./stored-task";
import { TaskId } from "./task-id";

export interface JobService {
  createJob(acx: AuthContext, kind: string, options: any): Promise<StoredTask>;

  getTask(acx: AuthContext, task: TaskId): Promise<StoredTask | undefined>;

  getTask(acx: AuthContext, job: JobId): Promise<StoredJob | undefined>;

  stopTask(acx: AuthContext, task: TaskId): Promise<StoredTask | undefined>;

  stopJob(acx: AuthContext, job: JobId): Promise<void>;

  pauseRunner(acx: AuthContext): Promise<void>;

  unpauseRunner(acx: AuthContext): Promise<void>;
}
