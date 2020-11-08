import { $Ucs2String } from "kryo/lib/ucs2-string.js";
import { WhiteListType } from "kryo/lib/white-list.js";

/**
 * A Dinoparc server.
 */
export type DinoparcServer = "dinoparc.com" | "en.dinoparc.com" | "sp.dinoparc.com";

export const $DinoparcServer: WhiteListType<DinoparcServer> = new WhiteListType({
  itemType: $Ucs2String,
  values: ["dinoparc.com", "en.dinoparc.com", "sp.dinoparc.com"],
});