import { PgAnnouncementService } from "@eternal-twin/announcement-pg";
import { ForumConfig } from "@eternal-twin/core/lib/forum/forum-config";
import { HttpRouter } from "@eternal-twin/core/lib/http/index";
import { DefaultLinkService } from "@eternal-twin/core/lib/link/service";
import { DefaultTwinoidService } from "@eternal-twin/core/lib/twinoid/service";
import { DefaultUserService } from "@eternal-twin/core/lib/user/service";
import { PgForumService } from "@eternal-twin/forum-pg";
import { Config } from "@eternal-twin/local-config";
import { PgAuthStore } from "@eternal-twin/native/lib/auth-store";
import { SystemClock } from "@eternal-twin/native/lib/clock";
import { Database as NativeDatabase } from "@eternal-twin/native/lib/database";
import { HttpDinoparcClient } from "@eternal-twin/native/lib/dinoparc-client";
import { PgDinoparcStore } from "@eternal-twin/native/lib/dinoparc-store";
import { JsonEmailFormatter } from "@eternal-twin/native/lib/email-formatter";
import { HttpHammerfestClient } from "@eternal-twin/native/lib/hammerfest-client";
import { PgHammerfestStore } from "@eternal-twin/native/lib/hammerfest-store";
import { PgLinkStore } from "@eternal-twin/native/lib/link-store";
import { MemMailer } from "@eternal-twin/native/lib/mailer";
import { PgOauthProviderStore } from "@eternal-twin/native/lib/oauth-provider-store";
import { ScryptPasswordService } from "@eternal-twin/native/lib/password";
import { NativeRestRouter } from "@eternal-twin/native/lib/rest";
import { NativeAuthService } from "@eternal-twin/native/lib/services/auth";
import { NativeDinoparcService } from "@eternal-twin/native/lib/services/dinoparc";
import { NativeHammerfestService } from "@eternal-twin/native/lib/services/hammerfest";
import { PgTokenStore } from "@eternal-twin/native/lib/token-store";
import { HttpTwinoidClient } from "@eternal-twin/native/lib/twinoid-client";
import { PgTwinoidStore } from "@eternal-twin/native/lib/twinoid-store";
import { PgUserStore } from "@eternal-twin/native/lib/user-store";
import { Uuid4Generator } from "@eternal-twin/native/lib/uuid";
import { createPgPool, Database } from "@eternal-twin/pg-db";

import { KoaAuth } from "../lib/helpers/koa-auth.js";
import { Api } from "../lib/index.js";

export async function createApi(config: Config): Promise<{ api: Api; teardown(): Promise<void>; nativeRouter: HttpRouter }> {
  const {pool, teardown: teardownPool} = createPgPool({
    host: config.db.host,
    port: config.db.port,
    name: config.db.name,
    user: config.db.user,
    password: config.db.password,
  });
  const nativeDatabase = await NativeDatabase.create({
    host: config.db.host,
    port: config.db.port,
    name: config.db.name,
    user: config.db.user,
    password: config.db.password,
  });

  const clock = new SystemClock();
  const uuidGenerator = new Uuid4Generator();
  const database = new Database(pool);
  const secretKeyStr: string = config.etwin.secret;
  const secretKeyBytes: Uint8Array = Buffer.from(secretKeyStr);
  const mailer = await MemMailer.create();
  const emailFormatter = await JsonEmailFormatter.create();
  const passwordService = ScryptPasswordService.withOsRng();
  const userStore = new PgUserStore({clock, database: nativeDatabase, databaseSecret: secretKeyStr, uuidGenerator});
  const dinoparcClient = new HttpDinoparcClient({clock});
  const dinoparcStore = await PgDinoparcStore.create({clock, database: nativeDatabase, uuidGenerator});
  const hammerfestStore = await PgHammerfestStore.create({clock, database: nativeDatabase, databaseSecret: secretKeyStr, uuidGenerator});
  const hammerfestClient = new HttpHammerfestClient({clock});
  const twinoidStore = new PgTwinoidStore({clock, database: nativeDatabase});
  const twinoidClient = new HttpTwinoidClient({clock});
  const linkStore = new PgLinkStore({clock, database: nativeDatabase});
  const link = new DefaultLinkService({dinoparcStore, hammerfestStore, linkStore, twinoidStore, userStore});
  const oauthProviderStore = await PgOauthProviderStore.create({clock, database: nativeDatabase, passwordService, uuidGenerator, secret: secretKeyStr});
  const authStore = await PgAuthStore.create({clock, database: nativeDatabase, uuidGenerator, secret: secretKeyStr});
  const auth = await NativeAuthService.create({authStore, clock, dinoparcClient, dinoparcStore, emailFormatter, hammerfestClient, hammerfestStore, linkStore, mailer, oauthProviderStore, passwordService, userStore, twinoidClient, twinoidStore, uuidGenerator, authSecret: secretKeyBytes});

  const koaAuth = new KoaAuth(auth);
  const forumConfig: ForumConfig = {
    postsPerPage: config.forum.postsPerPage,
    threadsPerPage: config.forum.threadsPerPage
  };
  const forum = new PgForumService(database, uuidGenerator, userStore, forumConfig);
  const announcement = new PgAnnouncementService({database, uuidGenerator, forum});

  const token = await PgTokenStore.create({clock, database: nativeDatabase, databaseSecret: secretKeyStr});
  const dinoparc = await NativeDinoparcService.create({dinoparcStore, linkStore, userStore});
  const hammerfest = await NativeHammerfestService.create({hammerfestClient, hammerfestStore, linkStore, userStore});
  const twinoid = new DefaultTwinoidService({twinoidStore, link});
  const user = new DefaultUserService({
    dinoparcClient,
    dinoparcStore,
    hammerfestStore,
    hammerfestClient,
    link,
    passwordService,
    token,
    twinoidStore,
    twinoidClient,
    userStore,
  });

  for (const [key, section] of config.forum.sections) {
    await forum.createOrUpdateSystemSection(
      key,
      {
        displayName: section.displayName,
        locale: section.locale,
      },
    );
  }

  const api: Api = {announcement, auth, clock, dev: null, forum, koaAuth, twinoid, user};
  const nativeRouter = await NativeRestRouter.create({dinoparc, hammerfest});

  async function teardown(): Promise<void> {
    await teardownPool();
    await nativeDatabase.close();
  }

  return {api, teardown, nativeRouter};
}

/**
 * Async resource manager for the Eternalfest API backend.
 *
 * @param config Server config
 * @param fn Inner function to call with an API pool.
 */
export async function withApi<R>(config: Readonly<Config>, fn: (api: Api) => Promise<R>): Promise<R> {
  const {api, teardown} = await createApi(config);
  try {
    return await fn(api);
  } finally {
    await teardown();
  }
}
