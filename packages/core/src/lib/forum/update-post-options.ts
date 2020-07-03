import { CaseStyle } from "kryo";
import { RecordIoType, RecordType } from "kryo/lib/record.js";

import { $NullableMarktwinText, NullableMarktwinText } from "../core/marktwin-text";
import { $NullableForumPostRevisionComment, NullableForumPostRevisionComment } from "./forum-post-revision-comment.js";
import { $ForumPostRevisionId, ForumPostRevisionId } from "./forum-post-revision-id.js";

export interface UpdatePostOptions {
  lastRevisionId: ForumPostRevisionId;
  content?: NullableMarktwinText;
  moderation?: NullableMarktwinText;
  comment: NullableForumPostRevisionComment;
}

export const $UpdatePostOptions: RecordIoType<UpdatePostOptions> = new RecordType<UpdatePostOptions>({
  properties: {
    lastRevisionId: {type: $ForumPostRevisionId},
    content: {type: $NullableMarktwinText, optional: true},
    moderation: {type: $NullableMarktwinText, optional: true},
    comment: {type: $NullableForumPostRevisionComment},
  },
  changeCase: CaseStyle.SnakeCase,
});
