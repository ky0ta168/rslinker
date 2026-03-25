int h_value = 100;

int multiply(int a, int b) {
    return a * b;
}

/* MinGW の CRT 初期化スタブ。
 * C ソース上の __main が COFF シンボル ___main になる。
 * foo.c の main 関数が参照する ___main をここで定義することで
 * DLL 検索を回避する。 */
void __main(void) {}
