import { Grammar } from "@eternal-twin/marktwin/lib/grammar";

export abstract class MarktwinService {
  readonly #marktwin: typeof import("@eternal-twin/marktwin");

  protected constructor(marktwin: typeof import("@eternal-twin/marktwin")) {
    this.#marktwin = marktwin;
  }

  public renderMarktwin(grammer: Grammar, input: string): string {
    return this.#marktwin.renderMarktwin(grammer, input);
  }
}
