import { Component, Input } from "@angular/core";
import { ShortForumPost } from "@eternal-twin/core/lib/forum/short-forum-post";

@Component({
  selector: "etwin-forum-post",
  templateUrl: "./forum-post.component.html",
  styleUrls: [],
})
export class ForumPostComponent {
  @Input()
  public post!: ShortForumPost;

  constructor() {
  }
}
