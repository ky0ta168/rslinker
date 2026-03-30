// main.c — counter.obj のグローバル変数・関数を使う
// 別の .obj で定義された g_count を参照するリロケーションが正しく機能するか確認する。

#include <windows.h>

void __main(void) {}

// counter.obj で定義されたシンボルを宣言
extern int g_count;
void increment(void);
int get_count(void);

int main(void) {
    // increment() 経由で変更
    increment();
    increment();
    increment();

    // get_count() 経由で読む
    int via_func = get_count();

    // g_count を直接読む (別 .obj のグローバル変数への直接参照)
    int via_global = g_count;

    char msg[128];
    wsprintfA(msg, "initial: 10\nafter 3x increment:\n  get_count() = %d\n  g_count     = %d",
              via_func, via_global);
    MessageBoxA(NULL, msg, "global var demo", MB_OK);
    ExitProcess(0);
    return 0;
}
