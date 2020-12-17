import { CaseStyle } from "kryo";
import { LiteralType } from "kryo/lib/literal.js";
import { RecordIoType, RecordType } from "kryo/lib/record.js";
import { $Ucs2String } from "kryo/lib/ucs2-string.js";

import { $DinoparcServer, DinoparcServer } from "../dinoparc-server.js";
import { $DinoparcUsername, DinoparcUsername } from "../dinoparc-username.js";

export interface InvalidDinoparcCredentials {
  name: "InvalidDinoparcCredentials";
  server: DinoparcServer;
  username: DinoparcUsername;
}

export const $InvalidDinoparcCredentials: RecordIoType<InvalidDinoparcCredentials> = new RecordType<InvalidDinoparcCredentials>({
  properties: {
    name: {type: new LiteralType({type: $Ucs2String, value: "InvalidDinoparcCredentials"})},
    server: {type: $DinoparcServer},
    username: {type: $DinoparcUsername},
  },
  changeCase: CaseStyle.SnakeCase,
});

export class InvalidDinoparcCredentialsError extends Error implements InvalidDinoparcCredentials {
  public name: "InvalidDinoparcCredentials";
  public server: DinoparcServer;
  public username: DinoparcUsername;

  public constructor(options: Omit<InvalidDinoparcCredentials, "name">) {
    const message: string = `Server: ${options.server}, username: ${JSON.stringify(options.username)}`;
    super(message);
    this.name = "InvalidDinoparcCredentials";
    this.server = options.server;
    this.username = options.username;
  }
}