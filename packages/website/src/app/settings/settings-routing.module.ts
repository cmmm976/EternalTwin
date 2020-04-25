import { Injectable, NgModule } from "@angular/core";
import { Resolve, Router, RouterModule, Routes } from "@angular/router";
import { AuthContext } from "@eternal-twin/etwin-api-types/lib/auth/auth-context";
import { AuthType } from "@eternal-twin/etwin-api-types/lib/auth/auth-type";
import { $CompleteUser, CompleteUser } from "@eternal-twin/etwin-api-types/lib/user/complete-user";
import { User } from "@eternal-twin/etwin-api-types/lib/user/user";
import { Observable, of as rxOf, throwError as rxThrowError } from "rxjs";
import { catchError as rxCatchError } from "rxjs/internal/operators/catchError";
import { first as rxFirst, map as rxMap, switchMap as rxSwitchMap } from "rxjs/operators";

import { AuthService } from "../../modules/auth/auth.service";
import { UserService } from "../../modules/user/user.service";
import { SettingsViewComponent } from "./settings-view.component";

@Injectable()
export class UserResolverService implements Resolve<CompleteUser | null> {
  private readonly auth: AuthService;
  private readonly user: UserService;

  constructor(auth: AuthService, router: Router, user: UserService) {
    this.auth = auth;
    this.user = user;
  }

  async resolve(): Promise<CompleteUser | null> {
    const completeCurrentUser$ = this.auth.auth().pipe(
      rxFirst(),
      rxSwitchMap((curUser: AuthContext) => {
        if (curUser.type !== AuthType.User) {
          return rxThrowError(new Error("Unauthenticated"));
        }
        return this.user.getUserById(curUser.userId);
      }),
      rxMap((user: User | CompleteUser | null): CompleteUser => {
        if (user === null || !$CompleteUser.test(user as any)) {
          throw new Error("AssertionError: Retrieving the current user should yield a complete user");
        }
        return user as CompleteUser;
      }),
      rxCatchError((err: Error): Observable<null> => {
        return rxOf(null);
      }),
    );
    return completeCurrentUser$.toPromise();
  }
}

const routes: Routes = [
  {
    path: "",
    component: SettingsViewComponent,
    pathMatch: "full",
    resolve: {
      user: UserResolverService,
    },
  },
];

@NgModule({
  imports: [RouterModule.forChild(routes)],
  exports: [RouterModule],
  providers: [UserResolverService],
})
export class SettingsRoutingModule {
}