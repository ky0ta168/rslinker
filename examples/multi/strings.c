// strings.c — 文字列ユーティリティ
// このファイルは strings.obj にコンパイルされ、main.obj とリンクされる。

#include <windows.h>

// "a + b = sum\na * b = product\na ^ b = pw" 形式で buf に書き込む。
// wsprintfA は user32.dll が直接エクスポートしており rslinker で使える。
void build_message(char* buf, int a, int b, int sum, int product, int pw) {
    wsprintfA(buf, "%d + %d = %d\n%d * %d = %d\n%d ^ %d = %d", a, b, sum, a, b, product, a, b, pw);
}
