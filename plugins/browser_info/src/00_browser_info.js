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
   * この関数は非同期で動作し、Promiseを返します。
   * @param {object} [params] - 取得するデータ量を制限するパラメータ。
   * @param {number} [params.historyLimit] - 取得する履歴の最大件数。
   * @param {number} [params.downloadLimit] - 取得するダウンロード情報の最大件数。
   * @returns {Promise<string>} ブラウザのコンテキストデータを含むJSON文字列のPromise。
   */
  getAllContextData: function (params) {
    // DenoのネイティブOp（Rustで実装された関数）を呼び出します。
    // 引数で渡されたparamsオブジェクトは、自動的にシリアライズされてRust側に渡されます。
    // パラメータがない場合はnullを渡します。
    return Deno.core.ops.op2_get_all_context_data(params || null);
  },
};
