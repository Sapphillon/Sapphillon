if(!globalThis.Sapphillon){globalThis.Sapphillon={};}
if(!globalThis.Sapphillon.Webscraper){globalThis.Sapphillon.Webscraper={};}
globalThis.Sapphillon.Webscraper.createInstance = function(params){
	return Deno.core.ops.op2_create_instance(params);
};
