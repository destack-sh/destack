// :NameValidation
export const VALID_NAME_REGEX: string = /^[a-zA-Z0-9_.\-:/ \xa0]*$/.source;
export const VALID_NAME_REGEXP = new RegExp(VALID_NAME_REGEX);

export const VALID_DESCRIPTION_REGEX: string = /^[a-zA-Z0-9_.\-:/&,;'"@ \xa0]*$/.source;
export const VALID_DESCRIPTION_REGEXP = new RegExp(VALID_DESCRIPTION_REGEX);

export const VALID_TEXT_REGEX: string = /^[a-zA-Z0-9_.\-+:/&,;'!?"@() \xa0]*$/.source;
export const VALID_TEXT_REGEXP = new RegExp(VALID_TEXT_REGEX);
