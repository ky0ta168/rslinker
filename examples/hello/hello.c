#include <windows.h>

// GCC は main() の先頭に __main() の呼び出しを挿入する。
// MinGW CRT なしでリンクするためダミーで定義する。
void __main(void) {}

int main(void) {
    MessageBoxA(NULL, "Hello from rslinker!", "rslinker Test", MB_OK);
    ExitProcess(0);
    return 0;
}
