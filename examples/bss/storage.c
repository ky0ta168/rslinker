// storage.c — 未初期化グローバル配列を持つ .obj
// g_array は初期値なし → .bss セクションに配置される。
// .bss はファイル上に実体を持たないが、仮想メモリ上は確保される。

int g_array[256]; // 256 * 4 = 1024 バイト、ファイルサイズには影響しない

void store(int idx, int val) {
    g_array[idx] = val;
}

int load(int idx) {
    return g_array[idx];
}
