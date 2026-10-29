// Types of i18n-status.mjs, for its test (the interface's TypeScript does
// not check plain JavaScript).

/** How complete one translation is. */
export interface LanguageStatus {
  tag: string;
  /** Its `language.name`, else its tag. */
  name: string;
  /** Texts in the source, a plural counting once. */
  total: number;
  translated: number;
  /** Keys of the texts it lacks, plurals without their `#category`. */
  missing: string[];
}

export function status(
  source: Record<string, string>,
  catalogs: Record<string, Record<string, string>>,
): LanguageStatus[];

export function report(languages: LanguageStatus[]): string;
