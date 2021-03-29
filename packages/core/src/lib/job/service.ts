import { AuthContext } from "../auth/auth-context";
import { StoredTask } from "./stored-task";
import { TaskId } from "./task-id";

export interface JobService {
  createTask(acx: AuthContext, kind: string, options: any): Promise<StoredTask>;

  getTask(acx: AuthContext, task: TaskId): Promise<StoredTask>;

  stopTask(acx: AuthContext, task: TaskId): Promise<StoredTask | undefined>;

  pauseRunner(acx: AuthContext): Promise<void>;

  unpauseRunner(acx: AuthContext): Promise<void>;
}
