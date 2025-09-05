if(!globalThis.Sapphillon){globalThis.Sapphillon={};}
if(!globalThis.Sapphillon.TabManager){globalThis.Sapphillon.TabManager={};}
globalThis.Sapphillon.TabManager.listTabs = function(){
	return Deno.core.ops.op2_list_tabs();
};
