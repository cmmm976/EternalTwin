import { Api, testAnnouncementService } from "@eternal-twin/announcement-test";
import { InMemoryAuthService } from "@eternal-twin/auth-in-memory";
import { Url } from "@eternal-twin/core/lib/core/url.js";
import { ForumConfig } from "@eternal-twin/core/lib/forum/forum-config.js";
import { LinkService } from "@eternal-twin/core/lib/link/service.js";
import { OauthProviderService } from "@eternal-twin/core/lib/oauth/provider-service.js";
import { InMemoryEmailService } from "@eternal-twin/email-in-memory";
import { JsonEmailTemplateService } from "@eternal-twin/email-template-json";
import { InMemoryForumService } from "@eternal-twin/forum-in-memory";
import { getLocalConfig } from "@eternal-twin/local-config";
import { VirtualClock } from "@eternal-twin/native/lib/clock.js";
import { MemDinoparcClient } from "@eternal-twin/native/lib/dinoparc-client.js";
import { MemDinoparcStore } from "@eternal-twin/native/lib/dinoparc-store.js";
import { MemHammerfestClient } from "@eternal-twin/native/lib/hammerfest-client.js";
import { MemHammerfestStore } from "@eternal-twin/native/lib/hammerfest-store.js";
import { MemLinkStore } from "@eternal-twin/native/lib/link-store.js";
import { MemTwinoidStore } from "@eternal-twin/native/lib/twinoid-store.js";
import { MemUserStore } from "@eternal-twin/native/lib/user-store.js";
import { Uuid4Generator } from "@eternal-twin/native/lib/uuid.js";
import { InMemoryOauthProviderStore } from "@eternal-twin/oauth-provider-in-memory";
import { ScryptPasswordService } from "@eternal-twin/password-scrypt";
import { HttpTwinoidClientService } from "@eternal-twin/twinoid-client-http";

import { MemAnnouncementService } from "../lib/index.js";

async function withMemAnnouncementService<R>(fn: (api: Api) => Promise<R>): Promise<R> {
  const config = await getLocalConfig();

  const clock = new VirtualClock();
  const uuidGenerator = new Uuid4Generator();
  const secretKeyStr: string = config.etwin.secret;
  const secretKeyBytes: Uint8Array = Buffer.from(secretKeyStr);
  const email = new InMemoryEmailService();
  const emailTemplate = new JsonEmailTemplateService(new Url("https://eternal-twin.net"));
  const password = new ScryptPasswordService();
  const dinoparcStore = new MemDinoparcStore({clock});
  const hammerfestStore = new MemHammerfestStore({clock});
  const twinoidStore = new MemTwinoidStore({clock});
  const dinoparcClient = new MemDinoparcClient({clock});
  const hammerfestClient = new MemHammerfestClient({clock});
  const twinoidClient = new HttpTwinoidClientService();
  const userStore = new MemUserStore({clock, uuidGenerator});
  const linkStore = new MemLinkStore({clock});
  const link = new LinkService({dinoparcStore, hammerfestStore, linkStore, twinoidStore, userStore});
  const oauthProviderStore = new InMemoryOauthProviderStore({clock, password, uuidGenerator});
  const oauthProvider = new OauthProviderService({
    clock,
    oauthProviderStore,
    tokenSecret: secretKeyBytes,
    userStore,
    uuidGenerator
  });
  const auth = new InMemoryAuthService({
    email,
    emailTemplate,
    dinoparcClient,
    dinoparcStore,
    hammerfestClient,
    hammerfestStore,
    link,
    oauthProvider,
    password,
    tokenSecret: secretKeyBytes,
    twinoidClient,
    twinoidStore,
    userStore,
    uuidGenerator
  });
  const forumConfig: ForumConfig = {postsPerPage: 10, threadsPerPage: 20};
  const forum = new InMemoryForumService(uuidGenerator, userStore, forumConfig);
  const announcement = new MemAnnouncementService({uuidGenerator, forum});

  return fn({auth, forum, announcement});
}

describe("MemAnnouncementService", function () {
  testAnnouncementService(withMemAnnouncementService);
});
