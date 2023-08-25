// :NameValidation
export const VALID_NAME_REGEX: string = /^[a-zA-Z0-9_.\- \xa0]*$/.source;
export const VALID_NAME_REGEXP = new RegExp(VALID_NAME_REGEX);
