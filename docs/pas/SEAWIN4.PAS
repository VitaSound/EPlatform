program win4;
uses graph,crt,mousem,winunit;

type
{объект для обработки событий}
     pMyHdl=^tMyHdl;
     tMyHdl=object(tHdl)
       constructor init;
       procedure handleevent(h:integer;param:string);virtual;
     end;
{переопределенный объект memo}
    pmymemo=^tmymemo;
    tmymemo=object(tmemo)
      function  handlemouse:integer;virtual;
      function    handlekey(c:char):integer;virtual;
      constructor init(px,py,sx,sy:integer;pntr:pHdl);
      procedure   show;virtual;
    end;
{максимальное количество точек}
const n=1000;
type
{запись для храниения координат точки}
  point=record
    x,y:real;
  end;
{запись для храния индексов точек треугольнока}
  triangle=record
    p1,p2,p3:word;
  end;
var
       points:array[1..n] of point;{массив записей точек}
       pntcnt:integer;{количество точек в массиве}
       tpntcnt:integer;{временная переменная}
       triangles:array[1..n div 3] of triangle;{массив записей треугольноков}
       trncnt:integer;{количество треугольников в массиве}
       filename:string;{имя файла}

var gd,gm,i,selitem:integer;
       hdl:pmyhdl;   {объект обработчик событий}
       menu:pmenu;   {объект главное меню}
       wrand:pwnd;   {объект окно случайного заполнения}
       wsave:pwnd;   {объект окно сохранения результата}
       wouttext:pwnd;{объект окно вывода на экран в текстовом виде}
       wload:pwnd;   {объект окно загузки данных из файла}
       wsavedt:pwnd; {объект окно сохранения исходных данных}
       c:char;       {символ с клавиатуры}
       f:text;       {файловая переменная для работы с файлами}
procedure addtria(pp1,pp2,pp3:word);
{добавление треугольника в список равносторонних треугольников}
var fl:boolean; i:integer;
begin
  fl:=true;
  {проверются все уже умеющиеся в массиве треугольники}
  for i:=1 to trncnt do
    if ((triangles[trncnt].p1=pp1)and(triangles[trncnt].p2=pp2)and(triangles[trncnt].p3=pp3))or
       ((triangles[trncnt].p1=pp1)and(triangles[trncnt].p3=pp2)and(triangles[trncnt].p2=pp3))or
       ((triangles[trncnt].p2=pp1)and(triangles[trncnt].p1=pp2)and(triangles[trncnt].p3=pp3))or
       ((triangles[trncnt].p2=pp1)and(triangles[trncnt].p3=pp2)and(triangles[trncnt].p1=pp3))or
       ((triangles[trncnt].p3=pp1)and(triangles[trncnt].p1=pp2)and(triangles[trncnt].p2=pp3))or
       ((triangles[trncnt].p3=pp1)and(triangles[trncnt].p2=pp2)and(triangles[trncnt].p1=pp3))
       then fl:=false;
  {и если треугольника, образованного 3 данными точками в }
  {массиве нет, то добавить его в массив}
  if fl then
    begin
      inc(trncnt);
      triangles[trncnt].p1:=pp1;
      triangles[trncnt].p2:=pp2;
      triangles[trncnt].p3:=pp3;
    end;
end;
function rasst(x1,y1,x2,y2:real):real;
{функция вычисления расстояния между двумя точками}
begin
  rasst:=sqrt(sqr(x2-x1)+sqr(y2-y1));
end;
function triangletest(x1,y1,x2,y2,x3,y3:real):boolean;
{функция, определяющая, является ли треугольник равносторонним}
begin
  triangletest:=(round(rasst(x1,y1,x2,y2))=round(rasst(x1,y1,x3,y3)))and
                (round(rasst(x1,y1,x2,y2))=round(rasst(x2,y2,x3,y3)));
end;

function  tmymemo.handlemouse:integer;
{функция обработки событий мыши}
begin
  inherited handlemouse;
end;
function   tmymemo.handlekey(c:char):integer;
{функция обработки событий клавиатуры}
begin
  inherited handlekey(c);
end;
constructor tmymemo.init(px,py,sx,sy:integer;pntr:pHdl);
{инициализация объекта}
begin
  inherited init(px,py,sx,sy,pntr);
end;
procedure   tmymemo.show;
{переопределение метода show для отображения текстовой информации}
var i:integer;
begin
{вызвать метод предка для отображения рамки и фона}
  inherited show;
{добавить текстовые данные}
  for i:=1 to trncnt do
    outtextxy(owner^.x+x+10+((i-1) div 32)*140,
    owner^.y+y+10+(((i-1) mod 32)*10),
    inttostr(i)+'-'+inttostr(triangles[i].p1)+','+inttostr(triangles[i].p2)+','+inttostr(triangles[i].p3));
end;

constructor tmyhdl.init;
begin
end;

procedure hideall;
{скрыть все окна и отобразить меню}
begin
        menu^.active:=false;
        wrand^.active:=false;
        wsave^.active:=false;
        wouttext^.active:=false;
        wload^.active:=false;
        wsavedt^.active:=false;
        menu^.show;
end;

procedure tmyhdl.handleevent(h:integer;param:string);
{обработчик событий}
var i,j,k:integer;
t1,t2:longint;
begin
  case h of
    1200:{отобразить окно загрузки из файла}
      begin
        h:=0;
        trncnt:=0;
        hideall;
        wload^.active:=true;
        wload^.show;
        filename:='d:\in.txt';
      end;
    1300:{отобразить окно случайного заполнения}
      begin
        h:=0;
        trncnt:=0;
        hideall;
        wrand^.active:=true;
        wrand^.show;
      end;
    2100:{расчет}
      begin
        h:=0;
        trncnt:=0;
        hideall;
        for i:=1 to pntcnt do
         begin
           for j:=i+1 to pntcnt do
             for k:=i+2 to pntcnt do
               if triangletest(points[i].x,points[i].y,
                               points[j].x,points[j].y,
                               points[k].x,points[k].y)
                  then addtria(i,j,k);
           setfillstyle(1,6);
           bar(0,460,50,480);
           setcolor(15);
           outtextxy(0,462,inttostr(round((i/pntcnt)*100))+'%');
         end;
        setfillstyle(1,6);
        bar(0,460,50,480);
        setcolor(15);
         outtextxy(0,462,'Все посчитано!');
      end;
    5001:{заполнить случайным образом}
      begin
        h:=0;
        hideall;
        pntcnt:=tpntcnt;
        randomize;
        for i:=1 to pntcnt do
          begin
            points[i].x:=random(300);
            points[i].y:=random(300);
          end;
        wrand^.active:=false;
        menu^.show;
        setcolor(15);
        outtextxy(0,460,'Массив заполнен случайными значениями в количестве '+inttostr(pntcnt));
      end;
   3100:{отобразить окно текстового вывода на экран}
      begin
        h:=0;
        hideall;
        wouttext^.active:=true;
        wouttext^.show;
      end;
   3200:{отобразить графически на экран}
      begin
        h:=0;
        hidemouse;
        for i:=1 to pntcnt do
          putpixel(round(points[i].x),round(points[i].y)+25,0);
        setcolor(15);
        for i:=1 to trncnt do
          begin
            line(round(points[triangles[i].p1].x),round(points[triangles[i].p1].y),
                 round(points[triangles[i].p2].x),round(points[triangles[i].p2].y));
            line(round(points[triangles[i].p1].x),round(points[triangles[i].p1].y),
                 round(points[triangles[i].p3].x),round(points[triangles[i].p3].y));
            line(round(points[triangles[i].p2].x),round(points[triangles[i].p2].y),
                 round(points[triangles[i].p3].x),round(points[triangles[i].p3].y));
          end;
        showmouse;
      end;
   3300:{отобразить окно сохранения данных в файл}
      begin
        h:=0;
        hideall;
        wsave^.active:=true;
        wsave^.show;
        filename:='d:\out.txt';
      end;
   3400:{отобразить окно сохранения исходных данных в файл}
      begin
        h:=0;
        hideall;
        wsavedt^.active:=true;
        wsavedt^.show;
        filename:='d:\data.txt';
      end;
   5002:{обработка нажатия кнопки ОТМЕНА}
      begin
        h:=0;
        hideall;
      end;
   5003:{обработка события ввода данных в текстовое окно}
      begin
        h:=0;
        tpntcnt:=strtoint(param);
      end;
   6001:{обработка нажатия кнопки СОХРАНИТЬ}
      begin
        h:=0;
        hideall;
        setcolor(15);
        outtextxy(0,460,'Данные сохранены в файл: '+filename);
        assign(f,filename);
        rewrite(f);
        writeln(f,'Номера точек, образующих равносторонние треугольники ');writeln(f);
        for i:=1 to trncnt do
          writeln(f,triangles[i].p1,' ',triangles[i].p2,' ',triangles[i].p3);
        close(f);
      end;
   6002:{обработка нажатия кнопки ОТМЕНА}
      begin
        h:=0;
        hideall;
      end;
   6003:{обработка события ввода данных в текстовое окно}
      begin
        h:=0;
        filename:=param;
      end;
   7001:{обработка нажатия кнопки ЗАКРЫТЬ}
      begin
        h:=0;
        hideall;
      end;
   8001:{обработка нажатия кнопки ЗАГРУЗИТЬ}
      begin
        h:=0;
        hideall;
        setcolor(15);
        assign(f,filename);
        reset(f);
        while not(eof(f))do
          begin
            inc(pntcnt);
            readln(f,points[pntcnt].x,points[pntcnt].y);
          end;
        close(f);
        outtextxy(0,460,'Данные загружены из файла: '+filename);
      end;
   8002:{обработка нажатия кнопки ОТМЕНА}
      begin
        h:=0;
        hideall;
      end;
   9001:{обработка нажатия кнопки СОХРАНИТЬ}
      begin
        h:=0;
        hideall;
        setcolor(15);
        assign(f,filename);
        rewrite(f);
        for i:=1 to pntcnt do
          writeln(f,points[i].x,' ',points[i].y);
        close(f);
        outtextxy(0,460,'Данные сохранены в файл: '+filename);
      end;
   9002:{обработка нажатия кнопки ОТМЕНА}
      begin
        h:=0;
        hideall;
      end;
   4100:{обработка нажатия пункта меню ВЫХОД}
      begin
        h:=0;
        c:=#27;
      end;
  end;
end;
{-------------------------------------------------------------------}

begin
  {количество точе к итреугольников в массивах = 0}
  pntcnt:=0;
  trncnt:=0;
  {инициализация графического режима}
  gd:=vga; gm:=vgahi;
  initgraph(gd,gm,'d:\tools\bp\bgi');
  {создание черно-белой палитры}
  for i:=0 to 15 do
      if i<8 then setrgbpalette(i,i*4,i*4,i*4)
             else setrgbpalette(48+i,i*4,i*4,i*4);
  setfillstyle(1,6);
  bar(0,0,640,480);
  {сознание объекта-обработчика событий}
  hdl:=new(pmyhdl,init);
  {создание объекта-главное меню}
  menu:=new(pmenu,init(Hdl));
  {добавление пунктов меню}
  menu^.addhitem('Ввод данных');
   menu^.addvitem(1,'Из файла',1200);
   menu^.addvitem(1,'Случайным образом',1300);
  menu^.addhitem('Расчет');
   menu^.addvitem(2,'Расчитать',2100);
  menu^.addhitem('Вывод результата');
   menu^.addvitem(3,'На экран-текст',3100);
   menu^.addvitem(3,'На экран-графически',3200);
   menu^.addvitem(3,'В файл-текст',3300);
   menu^.addvitem(3,'В файл-исходные данные',3400);
  menu^.addhitem('Выход');
   menu^.addvitem(4,'Выход из программы',4100);
  {отобразить меню}
  menu^.show;

  {создание объекта-окно случайного заполнения}
  {и создание графических элементов на нем}
  wrand:=new(pwnd,init(60,160,210,80,'Заполнить случайно',hdl));
  wrand^.insert(new(pbutton,init( 10,50,90,20,'Заполнить',5001,hdl)));
  wrand^.insert(new(pbutton,init(110,50,90,20,'Отмена',5002,hdl)));
  wrand^.insert(new(plabel,init(10,20,'Введите чесло точек:')));
  wrand^.insert(new(pedit,init(160,20,40,20,'0',5003,hdl,true)));

  {создание объекта-окно сохранения в файл}
  {и создание графических элементов на нем}
  wsave:=new(pwnd,init(60,160,210,100,'Сохранить в файл',hdl));
  wsave^.insert(new(pbutton,init( 10,70,90,20,'Сохранить',6001,hdl)));
  wsave^.insert(new(pbutton,init(110,70,90,20,'Отмена',6002,hdl)));
  wsave^.insert(new(plabel,init(10,20,'Введите имя файла:')));
  wsave^.insert(new(pedit,init(10,40,190,20,'out.txt',6003,hdl,false)));

  {создание объекта-окно вывода результата на экран}
  {и создание графических элементов на нем}
  wouttext:=new(pwnd,init(20,30,600,400,'Результат в текстовом виде',hdl));
  wouttext^.insert(new(pbutton,init( 10,370,90,20,'Закрыть',7001,hdl)));
  wouttext^.insert(new(pmymemo,init( 10,25,575,340,hdl)));

  {создание объекта-окно загрузки из файла}
  {и создание графических элементов на нем}
  wload:=new(pwnd,init(60,160,210,100,'Загрузить из файла',hdl));
  wload^.insert(new(pbutton,init( 10,70,90,20,'Загрузить',8001,hdl)));
  wload^.insert(new(pbutton,init(110,70,90,20,'Отмена',8002,hdl)));
  wload^.insert(new(plabel,init(10,20,'Введите имя файла:')));
  wload^.insert(new(pedit,init(10,40,190,20,'d:\in.txt',6003,hdl,false)));

  {создание объекта-окно сохранения в файл}
  {и создание графических элементов на нем}
  wsavedt:=new(pwnd,init(60,160,210,100,'Сохранить исходные данные в файл',hdl));
  wsavedt^.insert(new(pbutton,init( 10,70,90,20,'Сохранить',9001,hdl)));
  wsavedt^.insert(new(pbutton,init(110,70,90,20,'Отмена',9002,hdl)));
  wsavedt^.insert(new(plabel,init(10,20,'Введите имя файла:')));
  wsavedt^.insert(new(pedit,init(10,40,190,20,'data.txt',6003,hdl,false)));

  {отобразить курсор мыши}
  showmouse;
  {задать окно, в котором будет перемещаться мышь(весь экран)}
  {если этого не сделать, то мышь почему-то ниже середины не перемеается}
  asm
    mov ax,8
    mov cx,0
    mov dx,479
    int 33h
  end;
  {цикл полчучения и обработки событий}
  repeat
  {прочитать данные с мыши}
    readmouse;
    {если нажата кнопка мыши и курсор попадает в область главного меню}
    if (mask<>0)and(my<20)
            then begin
                   menu^.active:=true;
                   menu^.show;
                 end;
    {если какое-то из окон активно, то передать ему на обработку событие мыши}
    if menu^.active     then menu^.handlemouse;
    If wrand^.active    then if mask<>0 then wrand^.handlemouse;
    If wsave^.active    then if mask<>0 then wsave^.handlemouse;
    If wload^.active    then if mask<>0 then wload^.handlemouse;
    If wouttext^.active then if mask<>0 then wouttext^.handlemouse;
    If wsavedt^.active  then if mask<>0 then wsavedt^.handlemouse;
    {чтение информации с клавиатуры}
    if keypressed then
      begin
        c:=readkey;
        if c=#00 then c:=readkey;
        {если нажата кнопка F10 активизировать главное меню}
        if c=#68 then
          begin      {F10}
            menu^.active:=true;
            menu^.show;
          end;
        {если какое-то из окон активно, то передать событие ему на обработку}
        if wrand^.active then wrand^.handlekey(c);
        if menu^.active then menu^.handlekey(c);
        if wsave^.active then wsave^.handlekey(c);
        if wload^.active then wload^.handlekey(c);
        if wouttext^.active then wouttext^.handlekey(c);
        if wsavedt^.active then wsavedt^.handlekey(c);
      end;

  until c=#27;
  dispose(menu,done);
  closegraph;
end.
