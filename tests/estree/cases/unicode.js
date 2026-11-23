const café = "café";
const naïve = café;
const 日本語 = "日本語";
const e\u0301 = "combining mark";
// 😀 emoji before an identifier
const afterEmoji = `${café} 😀 ${日本語} ${e\u0301}`;
const astral = "😀😀";
const mixed = `${astral} café 日本語`;
