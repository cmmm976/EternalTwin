import { Component } from "@angular/core";

@Component({
  selector: "etwin-main-menu",
  templateUrl: "./main-menu.component.html",
  styleUrls: [],
})
export class MainMenuComponent {
  public npc_pick: number;
  constructor() {
    this.npc_pick = Math.floor(Math.random() * 11);
  }
}