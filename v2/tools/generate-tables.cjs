const fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const ts=require('../.qa/v1/node_modules/typescript');
const root=path.resolve(__dirname,'../..');
const sandbox={exports:{}};vm.runInNewContext(ts.transpileModule(fs.readFileSync(root+'/src/currency.tsx','utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS}}).outputText,sandbox);
const {currencyCode2Name:names,currencyCode2CountryAndRegion:countries}=sandbox.exports;
const rows=Object.keys(countries).map(code=>({code,name:names[code],country:countries[code],symbol:new Intl.NumberFormat('en-US',{style:'currency',currency:code,minimumFractionDigits:0,maximumFractionDigits:4}).formatToParts(0).find(p=>p.type==='currency').value}));
fs.writeFileSync(root+'/v2/src/currencies.json',JSON.stringify(rows,null,2)+'\n');
fs.writeFileSync(root+'/v2/.qa/intl-runtime.json',JSON.stringify({node:process.version,icu:process.versions.icu,count:rows.length},null,2));
