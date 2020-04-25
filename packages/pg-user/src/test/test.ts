import { AuthScope } from "@eternal-twin/etwin-api-types/lib/auth/auth-scope.js";
import { AuthType } from "@eternal-twin/etwin-api-types/lib/auth/auth-type.js";
import { GuestAuthContext } from "@eternal-twin/etwin-api-types/lib/auth/guest-auth-context.js";
import { RegisterWithUsernameOptions } from "@eternal-twin/etwin-api-types/lib/auth/register-with-username-options";
import { AuthService } from "@eternal-twin/etwin-api-types/lib/auth/service.js";
import { UserAndSession } from "@eternal-twin/etwin-api-types/lib/auth/user-and-session.js";
import { UserAuthContext } from "@eternal-twin/etwin-api-types/lib/auth/user-auth-context";
import { ObjectType } from "@eternal-twin/etwin-api-types/lib/core/object-type.js";
import { CompleteUser } from "@eternal-twin/etwin-api-types/lib/user/complete-user";
import { UserService } from "@eternal-twin/etwin-api-types/lib/user/service";
import { User } from "@eternal-twin/etwin-api-types/lib/user/user";
import { UserDisplayName } from "@eternal-twin/etwin-api-types/lib/user/user-display-name";
import { NullableUserRef } from "@eternal-twin/etwin-api-types/lib/user/user-ref";
import { Username } from "@eternal-twin/etwin-api-types/lib/user/username";
import chai from "chai";

export interface Api {
  auth: AuthService;
  user: UserService;
}

const GUEST_AUTH: GuestAuthContext = {type: AuthType.Guest, scope: AuthScope.Default};

async function createUser(
  auth: AuthService,
  username: Username,
  displayName: UserDisplayName,
  password: string,
): Promise<UserAuthContext> {
  const usernameOptions: RegisterWithUsernameOptions = {
    username,
    displayName,
    password: Buffer.from(password),
  };
  const userAndSession: UserAndSession = await auth.registerWithUsername(GUEST_AUTH, usernameOptions);
  return {
    type: AuthType.User,
    scope: AuthScope.Default,
    userId: userAndSession.user.id,
    displayName: userAndSession.user.displayName,
    isAdministrator: userAndSession.user.isAdministrator,
  };
}

export function testAuthService(withApi: (fn: (api: Api) => Promise<void>) => Promise<void>) {
  it("Register the admin and retrieve itself (ref)", async function (this: Mocha.Context) {
    this.timeout(30000);
    return withApi(async (api: Api): Promise<void> => {
      const aliceAuth: UserAuthContext = await createUser(api.auth, "alice", "Alice", "aaaaa");
      {
        const actual: NullableUserRef = await api.user.getUserRefById(aliceAuth, aliceAuth.userId);
        chai.assert.isNotNull(actual);
        const expected: NullableUserRef = {
          type: ObjectType.User,
          id: actual!.id,
          displayName: "Alice",
        };
        chai.assert.deepEqual(actual, expected);
      }
    });
  });

  it("Register the admin and retrieve itself (complete)", async function (this: Mocha.Context) {
    this.timeout(30000);
    return withApi(async (api: Api): Promise<void> => {
      const aliceAuth: UserAuthContext = await createUser(api.auth, "alice", "Alice", "aaaaa");
      {
        const actual: User | CompleteUser | null = await api.user.getUserById(aliceAuth, aliceAuth.userId);
        chai.assert.isNotNull(actual);
        chai.assert.instanceOf((actual as CompleteUser).ctime, Date);
        const expected: CompleteUser = {
          type: ObjectType.User,
          id: actual!.id,
          displayName: "Alice",
          isAdministrator: true,
          ctime: (actual as CompleteUser).ctime,
          username: "alice",
          emailAddress: null,
          hasPassword: true,
        };
        chai.assert.deepEqual(actual, expected);
      }
    });
  });

  it("Register an admin and user, retrieve the admin from the user", async function (this: Mocha.Context) {
    this.timeout(30000);
    return withApi(async (api: Api): Promise<void> => {
      const aliceAuth: UserAuthContext = await createUser(api.auth, "alice", "Alice", "aaaaa");
      const bobAuth: UserAuthContext = await createUser(api.auth, "bob", "Bob", "bbbbb");
      {
        const actual: User | CompleteUser | null = await api.user.getUserById(bobAuth, aliceAuth.userId);
        chai.assert.isNotNull(actual);
        const expected: User = {
          type: ObjectType.User,
          id: actual!.id,
          displayName: "Alice",
          isAdministrator: true,
        };
        chai.assert.deepEqual(actual, expected);
      }
    });
  });
}