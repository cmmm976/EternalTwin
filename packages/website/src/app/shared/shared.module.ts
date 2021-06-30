import { CommonModule } from "@angular/common";
import { NgModule } from "@angular/core";
import { FormsModule, ReactiveFormsModule } from "@angular/forms";
import { RouterModule } from "@angular/router";

import { EtwinBarComponent } from "./etwin-bar.component";
import { EtwinFooterComponent } from "./etwin-footer.component";
import { LanguagePickerComponent } from "./language-picker.component";
import { MainLayoutComponent } from "./main-layout.component";
import { MainMenuComponent } from "./main-menu.component";
import { PaginationComponent } from "./pagination.component";
import { SmallLayoutComponent } from "./small-layout.component";
import { SidebarLayoutComponent } from "./sidebar-layout.component";
import { UnlinkDinoparcButtonComponent } from "./unlink-dinoparc-button.component";
import { UnlinkHammerfestButtonComponent } from "./unlink-hammerfest-button.component";
import { UnlinkTwinoidButtonComponent } from "./unlink-twinoid-button.component";
import { UserLinkComponent } from "./user-link.component";

@NgModule({
  declarations: [
    EtwinBarComponent,
    EtwinFooterComponent,
    LanguagePickerComponent,
    MainLayoutComponent,
    MainMenuComponent,
    PaginationComponent,
    SmallLayoutComponent,
    SidebarLayoutComponent,
    UnlinkDinoparcButtonComponent,
    UnlinkHammerfestButtonComponent,
    UnlinkTwinoidButtonComponent,
    UserLinkComponent,
  ],
  imports: [
    CommonModule,
    FormsModule,
    ReactiveFormsModule,
    RouterModule.forChild([]),
  ],
  exports: [MainMenuComponent, MainLayoutComponent, SmallLayoutComponent, SidebarLayoutComponent, RouterModule, PaginationComponent, UserLinkComponent, UnlinkHammerfestButtonComponent, UnlinkDinoparcButtonComponent, UnlinkTwinoidButtonComponent],
})
export class SharedModule {
}
