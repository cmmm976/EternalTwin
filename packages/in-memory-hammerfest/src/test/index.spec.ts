import { AuthScope } from "@eternal-twin/etwin-api-types/lib/auth/auth-scope.js";
import { AuthType } from "@eternal-twin/etwin-api-types/lib/auth/auth-type.js";
import { GuestAuthContext } from "@eternal-twin/etwin-api-types/lib/auth/guest-auth-context.js";
import { ObjectType } from "@eternal-twin/etwin-api-types/lib/core/object-type.js";
import { HammerfestSession } from "@eternal-twin/etwin-api-types/lib/hammerfest/hammerfest-session.js";
import chai from "chai";

import { InMemoryHammerfestService } from "../lib/index.js";

const GUEST_AUTH: GuestAuthContext = {
  type: AuthType.Guest,
  scope: AuthScope.Default,
};

describe("InMemoryHammerfestService", () => {
  it("createSession", async () => {
    const hammerfest = new InMemoryHammerfestService();

    hammerfest.createUser("hammerfest.fr", 123, "alice", Buffer.from("aaaaa"));

    {
      const actual: HammerfestSession = await hammerfest.createSession(
        GUEST_AUTH,
        {server: "hammerfest.fr", login: "alice", password: Buffer.from("aaaaa")},
      );
      const expected: HammerfestSession = {
        ctime: actual.ctime,
        atime: actual.ctime,
        key: actual.key,
        user: {type: ObjectType.HammerfestUser, server: "hammerfest.fr", id: 123, login: "alice"},
      };
      chai.assert.deepEqual(actual, expected);
    }
  });
});