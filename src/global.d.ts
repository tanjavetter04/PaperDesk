declare module "*.aff?url" {
  const src: string;
  export default src;
}

declare module "*.dic?url" {
  const src: string;
  export default src;
}

declare module "*.dic?raw" {
  const src: string;
  export default src;
}

declare module "nspell" {
  type NSpellDict = { aff?: string; dic: string };
  export default class NSpell {
    constructor(dict: { aff: string; dic: string } | NSpellDict[]);
    correct(value: string): boolean;
    suggest(value: string): string[];
  }
}
