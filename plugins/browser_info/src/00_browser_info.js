// Copyright 2025 The Floorp Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

if (!globalThis.Sapphillon) {
  globalThis.Sapphillon = {};
}

// BrowserInfo用の名前空間を作成します。
globalThis.Sapphillon.BrowserInfo = {
  /**
   * ブラウザのコンテキストデータ（履歴、タブ、ダウンロード情報）を取得します。
   * 非同期の Deno op を呼び出します（Promise を返す）。
   * @param {object} [params]
   * @returns {Promise<string>}
   */
  getAllContextData: function (params) {
    const ops = (Deno.core && Deno.core.ops) || {};
    if (!ops.op2_get_all_context_data) {
      throw new Error("BrowserInfo op (op2_get_all_context_data) is未登録: プラグイン初期化前です");
    }
    return ops.op2_get_all_context_data(params || null);
  },
};
