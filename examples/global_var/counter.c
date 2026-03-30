// counter.c — グローバル変数を定義する .obj
// g_count は初期値あり → .data セクションに配置される。
// main.obj からこの変数を参照するとき、リロケーションが発生する。

int g_count = 10;

void increment(void) {
    g_count++;
}

int get_count(void) {
    return g_count;
}
