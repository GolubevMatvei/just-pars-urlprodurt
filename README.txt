just-pars-urlproduct 0.1.3
Golubev Matvei, golubevmt@gmail.com
Консольная утелята для парсинра url продуктов с сайта ptatel.ru

USAGE:
    just-pars-urlprodect [OPTIONS] <INPUT>

ARGS:
    <INPUT>    Путь до списка url ссылок на каталоги товаров

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -o, --output <DIR>    Путь, куда сохранять json файл. Например: ./urllist.json
    
Программа читает txt файл с сылками на каталоги товаров, обходит все страеици в каталоге и сохраняет товары в json файл.
Для карректрой работы ссылки необходимо копировать с афторизованной страници питптель.
Вы можете укачать путь, по которому будет сохранятся json файл флагом -o, --output. 

