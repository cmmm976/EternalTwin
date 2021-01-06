import { Component, ElementRef, OnInit, ViewChild } from "@angular/core";
import { FormControl, FormGroup, Validators } from "@angular/forms";
import { ActivatedRoute, Router } from "@angular/router";
import { HtmlText } from "@eternal-twin/core/lib/core/html-text.js";
import { $MarktwinText, MarktwinText } from "@eternal-twin/core/lib/core/marktwin-text";
import { ForumSection } from "@eternal-twin/core/lib/forum/forum-section";
import { ForumSectionId } from "@eternal-twin/core/lib/forum/forum-section-id";
import { ForumThread } from "@eternal-twin/core/lib/forum/forum-thread";
import { $ForumThreadTitle, ForumThreadTitle } from "@eternal-twin/core/lib/forum/forum-thread-title";
import { renderMarktwin } from "@eternal-twin/marktwin";
import { Grammar } from "@eternal-twin/marktwin/lib/grammar.js";
import { NEVER as RX_NEVER, Observable, Subscription } from "rxjs";
import { map as rxMap } from "rxjs/operators";

import { ForumService } from "../../modules/forum/forum.service";

const FORUM_SECTION_NOT_FOUND: unique symbol = Symbol("FORUM_SECTION_NOT_FOUND");

@Component({
  selector: "etwin-new-forum-thread",
  templateUrl: "./new-forum-thread.component.html",
  styleUrls: [],
})
export class NewForumThreadComponent implements OnInit {
  private readonly route: ActivatedRoute;
  private readonly forum: ForumService;
  private readonly router: Router;

  public section$: Observable<ForumSection | typeof FORUM_SECTION_NOT_FOUND>;
  public readonly FORUM_SECTION_NOT_FOUND = FORUM_SECTION_NOT_FOUND;
  public readonly $ForumThreadTitle = $ForumThreadTitle;

  public readonly newThreadForm: FormGroup;
  public readonly title: FormControl;
  public readonly body: FormControl;
  public readonly sectionId: FormControl;

  public pendingSubscription: Subscription | null;
  public serverError: Error | null;
  public preview: string;
  public showPreview: boolean;

  constructor(
    route: ActivatedRoute,
    forum: ForumService,
    router: Router,
  ) {
    this.route = route;
    this.section$ = RX_NEVER;
    this.forum = forum;
    this.router = router;

    this.title = new FormControl(
      "",
      [Validators.required, Validators.minLength($ForumThreadTitle.minLength ?? 0), Validators.maxLength($ForumThreadTitle.maxLength)],
    );
    this.body = new FormControl(
      "",
      [Validators.required, Validators.minLength($MarktwinText.minLength ?? 0), Validators.maxLength($MarktwinText.maxLength)],
    );
    this.sectionId = new FormControl(
      "",
      [Validators.required],
    );
    this.newThreadForm = new FormGroup({
      title: this.title,
      body: this.body,
      sectionId: this.sectionId,
    });
    this.pendingSubscription = null;
    this.serverError = null;
    this.showPreview = false;
    this.preview = "";
  }

  ngOnInit(): void {
    interface RouteData {
      section: ForumSection | null;
    }

    const routeData$: Observable<RouteData> = this.route.data as any;
    this.section$ = routeData$.pipe(rxMap(({section}: RouteData): ForumSection | typeof FORUM_SECTION_NOT_FOUND => {
      if (section !== null) {
        this.sectionId.setValue(section.id);
        return section;
      } else {
        return FORUM_SECTION_NOT_FOUND;
      }
    }));
  }

  public toggleStyle(textArea: HTMLInputElement, style: string) {
    const styleToMarktwin: Map<string, string> = new Map([
      ["strong", "**"],
      ["emphasis", "_"],
    ]);

    const mark: string = styleToMarktwin.get(style) || "";
    const value: string = this.body.value;
    const start: number = textArea.selectionStart || 0;
    const end: number = textArea.selectionEnd || 0;
    const length: number = end - start;

    if (value.substr(start - mark.length, mark.length) === value.substr(end, mark.length)
        && value.substr(end, mark.length) === mark) {
      this.body.setValue(
        value.substr(0, start - mark.length) + value.substr(start, length) + value.substr(end + mark.length)
      );

      textArea.selectionStart = start - mark.length;
      textArea.selectionEnd = end - mark.length;
    } else if (value.substr(start, mark.length) === value.substr(end - mark.length, mark.length)
               && value.substr(start, mark.length) === mark) {
      this.body.setValue(
        value.substr(0, start) + value.substr(start + mark.length, length - mark.length * 2) + value.substr(end)
      );

      textArea.selectionStart = start;
      textArea.selectionEnd = end - mark.length * 2;
    } else {
      this.body.setValue(
        value.substr(0, start) + mark + value.substr(start, length) + mark + value.substr(end)
      );

      textArea.selectionStart = start + mark.length;
      textArea.selectionEnd = end + mark.length;
    }

    textArea.focus();
  }

  public onShowPreview() {
    const mktGrammar: Grammar = {
      admin: false,
      depth: 4,
      emphasis: true,
      icons: ["etwin"],
      links: ["http", "https"],
      mod: true,
      quote: false,
      spoiler: false,
      strikethrough: true,
      strong: true,
    };

    const htmlBody: HtmlText = renderMarktwin(mktGrammar, this.body.value);
    this.preview = htmlBody;
  }

  public onSubmit(event: Event) {
    event.preventDefault();
    if (this.pendingSubscription !== null) {
      return;
    }
    const model: any = this.newThreadForm.getRawValue();
    const title: ForumThreadTitle = model.title;
    const body: MarktwinText = model.body;
    const sectionId: ForumSectionId = model.sectionId;
    const thread$ = this.forum.createThread(sectionId, {title, body});
    this.serverError = null;
    const subscription: Subscription = thread$.subscribe({
      next: (thread: ForumThread): void => {
        subscription.unsubscribe();
        this.pendingSubscription = null;
        this.router.navigate(["", "forum", "threads", thread.id]);
      },
      error: (err: Error): void => {
        subscription.unsubscribe();
        this.pendingSubscription = null;
        this.serverError = err;
      },
      complete: (): void => {
        subscription.unsubscribe();
        this.pendingSubscription = null;
      },
    });
    this.pendingSubscription = subscription;
  }
}
