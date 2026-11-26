tab-catalogue = Каталог
tab-libraries = Бібліотеки
tab-exclusions = Винятки
tab-log = Журнал

menu-title = Меню
addons-folder = Тека AddOns
addons-folder-none = Не вибрана
no-folder-title = Теку AddOns не вибрано
change-folder-title = Вибір нової теки AddOns
no-folder-question = Гра шукає аддони в теці AddOns — зазвичай це { $example }. Запустіть гру хоча б раз, щоб вона створила ці теки.
no-folder-warning = Вибирайте саме теку з назвою AddOns — менеджер видаляє з неї всі незнайомі йому файли.
no-folder-choose = Вибрати теку
menu-language = Мова
menu-choose-folder = Змінити
menu-select-folder = Вибрати
scan-folder = Сканувати
menu-open-folder = Відкрити
menu-app-folders = Теки застосунку
menu-configuration = Конфігурація
menu-cache = Кеш застосунку
menu-custom = Власні аддони
log-to-file = Писати у файл
log-open-folder = Відкрити теку
log-clear = Видалити журнали
log-clear-title = Видалити файли журналів?
log-clear-all-disable = Все й вимкнути запис
log-clear-all = Усі файли журналів
log-clear-others = Минулі запуски
sync-on-launch = Синхронізація під час запуску
sync = Синхронізувати

loading = Завантаження…
loading-catalogue = Завантаження каталогу…
catalogue-failed = Не вдалося завантажити каталог: { $error }
retry = Повторити

search-placeholder = Пошук за назвою, автором або текою…
addon-count =
    { $count ->
        [one] { $count } аддон
        [few] { $count } аддони
       *[many] { $count } аддонів
    }
filter = Фільтр
filter-installed = Встановлені
filter-dependencies = Залежності
filter-favourites = Улюблене
sort-by = Сортування
sort-name = Назва
sort-author = Автор
sort-updated = Дата оновлення
sort-downloads = Усього завантажень
sort-monthly-downloads = Завантажень за місяць
sort-favourites = Улюблене
sort-status = Статус встановлення

addon-version = версія { $version }
add-favourite = Додати до улюбленого
remove-favourite = Прибрати з улюбленого
install = Встановити
install-directly = Встановлено автоматично — натисніть, щоб встановити напряму
required-by = Потребують:
remove = Видалити
open-page = Відкрити на esoui.com
select-addon = Виберіть аддон, щоб побачити його опис
description-failed = Не вдалося завантажити опис: { $error }
description-render-failed = Не вдалося показати опис
tab-description = Опис
tab-changelog = Список змін
screenshot-failed = Не вдалося завантажити знімок екрана
previous = НАЗАД
next = ДАЛІ

libraries-none = Збережених виборів поки немає. Коли одну теку надають кілька аддонів, ваш вибір з’явиться тут.
reset = Скинути
recommended = Рекомендовано

exclusions-custom = Власні винятки — один регулярний вираз на рядок
exclusions-automatic = Автоматично (зі встановлених аддонів)
exclusions-automatic-none = Немає: жодному встановленому аддону вони не потрібні.
clear = Очистити
save = Зберегти

ok = Гаразд
cancel = Скасувати
continue = Продовжити
open-link-question = Відкрити це посилання у браузері?
open-link-confirm = Відкрити
providers-title = Виберіть аддони
providers-note = Ці теки є в кількох аддонах. Виберіть, який використовувати для кожної теки.
candidate-folders = Теки: { $folders }
scan-prompt-title = Просканувати теку на наявність аддонів?
scan-prompt-question = Проаналізувати цю теку на наявність встановлених аддонів і додати їх до списку?
scan-prompt-warning = Увага: визначення може бути неточним. Спершу зробіть резервну копію теки — її вміст може бути перезаписано під час наступної синхронізації.
scan-keep-folder = Ні, не змінювати теку
scan-skip = Ні, не сканувати
scan-confirm = Так, сканувати
scan-complete = Сканування завершено
scan-failed = Сканування не вдалося
scan-added = Додані аддони
scan-unrecognised = Нерозпізнані теки
scan-nothing-new = У теці не знайдено нових аддонів.
catalogue-unavailable = Каталог недоступний.
catalogue-not-loaded = Каталог ще не завантажено.

sync-complete = Синхронізацію завершено
sync-failed = Синхронізація не вдалася
report-problems = Проблеми
sync-installed-addons = Встановлені аддони
sync-updated-addons = Оновлені аддони
sync-removed-addons = Видалені аддони
sync-installed-dependencies = Встановлені залежності
sync-updated-dependencies = Оновлені залежності
sync-removed-dependencies = Видалені залежності
sync-downloaded-again = { $version } (завантажено повторно)
sync-not-applied = Теку AddOns не змінено, бо не всі завантаження вдалися.
sync-download-failed = Не вдалося завантажити { $name }: { $error }
sync-missing-dependency = Не знайдено залежність: { $directory }
sync-unknown-addon = Немає в каталозі: { $name } (UID: { $uid })
remove-missing-title = Деякі аддони більше не в каталозі
remove-missing-question = Ці аддони було встановлено з каталогу, але тепер їх у ньому немає (можливо, тимчасово). Якщо продовжити синхронізацію, їхні файли буде видалено, а самі аддони — прибрано зі встановлених та обраного.
remove-missing-section = Немає в каталозі
remove-missing-uid = UID: { $uid }

menu-support = Підтримка
menu-report-problem = Повідомити про проблему
report-title = Повідомити про проблему
report-intro = Виберіть, що додати. Усе за бажанням.
report-include-config = Налаштування (app.ini)
report-include-config-backups = Резервні копії налаштувань
report-include-logs = Поточний журнал
report-include-log-files = Усі файли журналів
report-description-label = Що сталося?
report-description-placeholder = Опишіть проблему (необов’язково)
report-contact-label = Як з вами зв’язатися?
report-contact-placeholder = Discord, нік у грі — будь-що (необов’язково)
report-send = Надіслати
report-save = Зберегти
report-sent-title = Звіт надіслано
report-sent-message = Дякуємо! Ваш звіт надіслано розробнику.
report-failed-title = Не вдалося надіслати
report-failed = Не вдалося надіслати звіт: { $error }
report-open-folder = Відкрити теку
report-save-failed-title = Не вдалося зберегти
report-save-failed = Не вдалося зберегти звіт: { $error }
report-saved-title = Звіт збережено
-report-hand-off = Ви можете надіслати цей файл розробнику.
report-saved-message = Збережено у { $path }. { -report-hand-off }
report-saved-sent-failed = Надіслати не вдалося, тому звіт збережено у { $path }. { -report-hand-off }
sync-up-to-date = Усе актуально.

log-empty = Журнал порожній…
migrate-title = Яку копію залишити?
migrate-question = Ваші налаштування та аддони знайдено і в старій теці, і в новому місці профілю. Виберіть, яку копію залишити — вибір застосується до всіх конфліктних файлів.
migrate-keep-old = Залишити стару теку
migrate-keep-profile = Залишити профіль
migrate-open-folders = Відкрити теки

menu-updates = Оновлення
menu-check-on-launch = Перевіряти при запуску
menu-notify-prereleases = Включати пре-релізи
menu-check-now = Перевірити зараз
update-title = Оновлення
update-available = Доступна нова версія.
update-open-page = Сторінка завантаження
update-dont-ask = Більше не питати
update-up-to-date = У вас найновіша версія.
update-check-failed = Не вдалося перевірити оновлення.

menu-experimental = Експериментальне
menu-discontinued = Застарілі аддони
menu-ttc-prices = Ціни TTC
menu-update-prices = Оновлювати ціни
region-eu = Європа
region-na = Америка
ttc-updated = Ціни TTC оновлено
ttc-failed = Не вдалося оновити ціни TTC: { $error }
