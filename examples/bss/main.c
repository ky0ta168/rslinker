// main.c — .bss に配置されたグローバル配列を読み書きする
// SectionLayout ダンプで .bss の RawSize=0 / VirtualSize>0 を確認できる。

#include <windows.h>

void __main(void) {}

void store(int idx, int val);
int load(int idx);

int main(void) {
    store(0, 42);
    store(1, 100);
    store(255, 999);

    int a = load(0);
    int b = load(1);
    int c = load(255);

    char msg[128];
    wsprintfA(msg, "load(0)   = %d\nload(1)   = %d\nload(255) = %d", a, b, c);
    MessageBoxA(NULL, msg, "bss demo", MB_OK);
    ExitProcess(0);
    return 0;
}
