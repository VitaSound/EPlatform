unit winunit;
{модуль, в котором содержатся визуальные компоненты: меню, окно, кнопка,}
{окно текстового ввода, окно для вывода многострочного текста}
interface
uses graph,crt,mousem;

type
     {объект с абстрактным методом, который необходимо переопределить}
     {для того, чтобы описать действия при получения определенного}
     {кода события}
     pHdl=^tHdl;
     tHdl=object
       {процедура обработки события по заданному коду}
       procedure handleevent(h:integer;param:string);virtual;
       {h-код события; param-дополнительные данные, например введенный текст}
     end;
    {элемент вертикального меню}
     smenuitem=record
       {координаты и размер}
       x,y,sizex,sizey:integer;
       {текст}
       text:string;
       {код для обработки события}
       code:integer;
     end;
    {элемент горизоньального меню}
     menuitem=record
       {координаты и размер}
       x,y,sizex,sizey:integer;
       {текст}
       text:string;
       {количество элементов меню}
       menuitems:integer;
       {размер пункта}
       msize:integer;
       {выбранный элемент}
       vmenusel:integer;
       {элементы}
       vmenu:array[1..5] of smenuitem;
     end;
    {объект меню}
    pmenu=^tmenu;
    tmenu=object
      {указатель на обработчик событий, ему передаются коды событий}
      ndlpntr:pHdl;
      {массив элементов горизонтального меню}
      hmenu:array[1..4]of menuitem;
      {количество элементов}
      menuitems:integer;
      {выбранное горизонтальное меню}
      hmenusel:integer;
      {признак необходимости отображать развернутыми вертикальные меню}
      ssub:boolean;
      {активность главного меню}
      active:boolean;
      {конструктор объекта}
      constructor init(pntr:pHdl);
      {добавление горизонтального меню}
      procedure   addhitem(txt:string);
      {добавление вертикального меню}
      procedure   addvitem(nm:integer;txt:string;cd:integer);
      {функция обработки события клавиатуры}
      function    handlekey(c:char):integer;
      {функция обработки события мыши}
      function    handlemouse:integer;
      {процедуры прорисовки меню}
      procedure   show;
      {деструктор объекта}
      destructor  done;
    end;
    {объект, наследниками которого будут все графические элементы}
    pgroup=^tgroup;
    tgroup=object
      {указатель на обработчик событий, ему передаются коды событий}
      ndlpntr:pHdl;
      {указатель на предка}
      owner:pgroup;
      {координаты и размер}
      x,y,sizex,sizey:integer;
      {следующий элемент группы}
      next:pgroup;
      {необходимость перерисовки}
      repaint:boolean;
      {процедура прорисовки}
      procedure show;virtual;
      {функция обработки события клавиатуры}
      function  handlemouse:integer;virtual;
      {функция обработки события мыши}
      function    handlekey(c:char):integer;virtual;
    end;
    {объект, определяющий окно}
    pwnd=^twnd;
    twnd=object(tGroup)
      {заголовок окна}
      title:string;
      {активность окна}
      active:boolean;
      {указатели на первый и последний элемент на окне}
      first,last:pgroup;
      {конструктор окна, включает координаты, размер, заголовок и указатель на}
      {обработчик событий}
      constructor init(px,py,sx,sy:integer;ttl:string;pntr:pHdl);
      {прорисовка окна}
      procedure   show;virtual;
      {обработка событий}
      function  handlemouse:integer;virtual;
      function    handlekey(c:char):integer;virtual;
      {добавление элемента на окне}
      procedure   insert(pp:pgroup);
      {деструктор окна}
      destructor  done;
    end;
    {объект-кнопка}
    pbutton=^tbutton;
    tbutton=object(tgroup)
      {активность}
      active:boolean;
      {надпись на кнопке}
      caption:string;
      {идентификатор кнопки}
      id:integer;
      {обработка событий}
      function  handlemouse:integer;virtual;
      function    handlekey(c:char):integer;virtual;
      {создание кнопки, включает координаты, размер, надпись, идентификатор}
      {указатель на обработчик}
      constructor init(px,py,sx,sy:integer;ttl:string;i:integer;pntr:pHdl);
      {прорисовка}
      procedure   show;virtual;
    end;
    {объект-текстовая метка}
    plabel=^tlabel;
    tlabel=object(tgroup)
      {надпись на метке}
      caption:string;
      {обработка событий}
      function  handlemouse:integer;virtual;
      function    handlekey(c:char):integer;virtual;
      {создание метки, включает координаты и надпись}
      constructor init(px,py:integer;ttl:string);
      procedure   show;virtual;
    end;
    {объект-поле текстового ввода}
    pedit=^tedit;
    tedit=object(tgroup)
      {активность}
      active:boolean;
      {текст в поле}
      text:string;
      {идентификатор}
      id:integer;
      {признак ввода только цифр}
      digitonly:boolean;
      {обработка событий}
      function  handlemouse:integer;virtual;
      function    handlekey(c:char):integer;virtual;
      {инициализация, включает координаты, размер, текст, идентификатор}
      {указатель на обработчик, признак ввода только цифр}
      constructor init(px,py,sx,sy:integer;ttl:string;i:integer;pntr:pHdl;dig:boolean);
      {прорисовка}
      procedure   show;virtual;
    end;
    {объект-окно текстового вывода}
    pmemo=^tmemo;
    tmemo=object(tgroup)
      active:boolean;
      function  handlemouse:integer;virtual;
      function    handlekey(c:char):integer;virtual;
      constructor init(px,py,sx,sy:integer;pntr:pHdl);
      procedure   show;virtual;
    end;

{функция проеобразования целого числа в строку}
function inttostr(i:integer):string;
{функция преобразования чтроки в целое число}
function strtoint(s:string):integer;


implementation

{функция проеобразования целого числа в строку}
function inttostr(i:integer):string;
var ts:string; a,b,c:integer;
begin
  a:=i div 100;
  i:=i-(a*100);
  b:=i div 10;
  i:=i-(b*10);
  c:=i;
  ts:=chr(ord('0')+a)+chr(ord('0')+b)+chr(ord('0')+c);
  inttostr:=ts;
end;
{функция преобразования чтроки в целое число}
function strtoint(s:string):integer;
var i,r:integer;
begin
  r:=0;
  for i:=1 to length(s) do
    r:=r*10+(ord(s[i])-ord('0'));
  strtoint:=r;
end;
{tmenu}
procedure tgroup.show;
begin
end;
function tgroup.handlemouse:integer;
begin
end;
function tgroup.handlekey(c:char):integer;
begin
end;
constructor tmenu.init(pntr:pHdl);
begin
  ndlpntr:=pntr;
  active:=false;
  menuitems:=0;hmenusel:=0;ssub:=false;
end;
{добавление горизонтального меню}
procedure tmenu.addhitem(txt:string);
begin
  hmenusel:=1;
  inc(menuitems);
  hmenu[menuitems].text:=txt;
  hmenu[menuitems].menuitems:=0;
  hmenu[menuitems].vmenusel:=0;
  hmenu[menuitems].msize:=0;
  if menuitems=1
  then hmenu[menuitems].x:=1
  else hmenu[menuitems].x:=hmenu[menuitems-1].x+hmenu[menuitems-1].sizex+2;
  hmenu[menuitems].y:=1;
  hmenu[menuitems].sizey:=18;
  hmenu[menuitems].sizex:=length(txt)*8+4;
end;
{добавление вертикального меню}
procedure tmenu.addvitem(nm:integer;txt:string;cd:integer);
var i:integer;
begin
  inc(hmenu[nm].menuitems);
  hmenu[nm].vmenu[hmenu[nm].menuitems].code:=cd;
  hmenu[nm].vmenu[hmenu[nm].menuitems].text:=txt;
  hmenu[nm].vmenu[hmenu[nm].menuitems].x:=hmenu[nm].x;
  if hmenu[nm].menuitems=1
  then hmenu[nm].vmenu[hmenu[nm].menuitems].y:=21
  else hmenu[nm].vmenu[hmenu[nm].menuitems].y:=hmenu[nm].vmenu[hmenu[nm].menuitems-1].y
  +hmenu[nm].vmenu[hmenu[nm].menuitems-1].sizey+1;
  if (length(txt)*8+4)>hmenu[nm].msize then
    begin
      hmenu[nm].msize:=(length(txt)*8+4);
      for i:=1 to hmenu[nm].menuitems do hmenu[nm].vmenu[i].sizex:=hmenu[nm].msize;
    end;
  hmenu[nm].vmenu[hmenu[nm].menuitems].sizex:=hmenu[nm].msize;
  hmenu[nm].vmenu[hmenu[nm].menuitems].sizey:=hmenu[nm].sizey;
end;
procedure tmenu.show;
var i,j:integer;
begin
  hidemouse;
{  if (ssub) then
    begin}
      setfillstyle(1,6);
      bar(0,0,640,480);
{    end;}
  setfillstyle(1,10);
  bar(0,0,640,20);
  for i:=1 to menuitems do
    begin
      if (active)and(hmenusel=i)
      then setfillstyle(1,15)
      else setfillstyle(1,0);
      bar(hmenu[i].x,hmenu[i].y,
      hmenu[i].x+hmenu[i].sizex,
      hmenu[i].y+hmenu[i].sizey);
      if (active)and(hmenusel=i)
      then setfillstyle(1,0)
      else setfillstyle(1,15);
      bar(hmenu[i].x+1,hmenu[i].y+1,
      hmenu[i].x+hmenu[i].sizex-1,
      hmenu[i].y+hmenu[i].sizey-1);
      if (active)and(hmenusel=i)
      then setcolor(15)
      else setcolor(0);
      settextstyle(2,0,5);
      outtextxy(hmenu[i].x+4,hmenu[i].y+5,hmenu[i].text);
      if (active)and(ssub)and(hmenusel=i) then
        begin
          for j:=1 to hmenu[i].menuitems do
            begin
              if hmenu[i].vmenusel=j
              then setfillstyle(1,15)
              else setfillstyle(1,0);
              bar(hmenu[i].vmenu[j].x,hmenu[i].vmenu[j].y,
              hmenu[i].vmenu[j].x+hmenu[i].vmenu[j].sizex,
              hmenu[i].vmenu[j].y+hmenu[i].vmenu[j].sizey);
              if hmenu[i].vmenusel=j
              then setfillstyle(1,0)
              else setfillstyle(1,15);
              bar(hmenu[i].vmenu[j].x+1,hmenu[i].vmenu[j].y+1,
              hmenu[i].vmenu[j].x+hmenu[i].vmenu[j].sizex-1,
              hmenu[i].vmenu[j].y+hmenu[i].vmenu[j].sizey-1);
              if hmenu[i].vmenusel=j
              then setcolor(15)
              else setcolor(0);
              outtextxy(hmenu[i].vmenu[j].x+4,hmenu[i].vmenu[j].y+5,hmenu[i].vmenu[j].text);
            end;
        end;
    end;
  showmouse;
end;
function   tmenu.handlekey(c:char):integer;
begin
  handlekey:=0;
        if c=#77 then
          begin      {right}
            if hmenusel<>0 then inc(hmenusel);
            if hmenusel>menuitems then hmenusel:=1;
            hmenu[hmenusel].vmenusel:=0;
            show;
          end;
        if c=#75 then
          begin      {left}
            if hmenusel<>0 then dec(hmenusel);
            if hmenusel=0 then  hmenusel:=menuitems;
            hmenu[hmenusel].vmenusel:=0;
            show;
          end;
        if c=#72 then
          begin      {up}
            if ssub then
              begin
               dec(hmenu[hmenusel].vmenusel);
               if hmenu[hmenusel].vmenusel<0
                 then hmenu[hmenusel].vmenusel:=0;
               show;
              end;
          end;
        if c=#80 then
          begin      {down}
            if ssub then
              begin
               inc(hmenu[hmenusel].vmenusel);
               if hmenu[hmenusel].vmenusel>hmenu[hmenusel].menuitems
                 then hmenu[hmenusel].vmenusel:=hmenu[hmenusel].menuitems;
               show;
              end else
              begin
                ssub:=true;
                inc(hmenu[hmenusel].vmenusel);
                show;
              end;
          end;
        if c=#13 then
          begin      {enter}
            if ssub then
              begin
                handlekey:=hmenu[hmenusel].vmenu[hmenu[hmenusel].vmenusel].code;
                active:=false;
                ssub:=false;
                show;
                ndlpntr^.handleevent(hmenu[hmenusel].vmenu[hmenu[hmenusel].vmenusel].code,'');
              end else
              begin
                ssub:=true;
                inc(hmenu[hmenusel].vmenusel);
                show;
              end;

          end;
end;
function  tmenu.handlemouse:integer;
var i,j,tm:integer; ts:boolean;
begin
  handlemouse:=0;
  if active then
    begin
      for i:=1 to menuitems do
        if (mx>=hmenu[i].x)and(my>=hmenu[i].y)and
           (mx<=hmenu[i].x+hmenu[i].sizex)and
           (my<=hmenu[i].y+hmenu[i].sizey)
         then
          begin
            if i<>hmenusel then
              begin
                hmenusel:=i;
                show;
              end;
          end;
      for i:=1 to hmenu[hmenusel].menuitems do
        if (mx>=hmenu[hmenusel].vmenu[i].x)and(my>=hmenu[hmenusel].vmenu[i].y)and
           (mx<=hmenu[hmenusel].vmenu[i].x+hmenu[hmenusel].vmenu[i].sizex)and
           (my<=hmenu[hmenusel].vmenu[i].y+hmenu[hmenusel].vmenu[i].sizey)
         then
          begin
            if i<>hmenu[hmenusel].vmenusel then
              begin
                hmenu[hmenusel].vmenusel:=i;
                show;
              end;
          end;

    end; {else
    begin
      active:=false;
      ssub:=false;
      show;
    end;}
    if (mask<>0)and(my<=20)then
      begin
        ssub:=true;
        show;
      end;
  if (active)and(mask<>0)and(my>20) then
    begin
      ts:=true;
      for j:=1 to menuitems do
      for i:=1 to hmenu[j].menuitems do
        if (j=hmenusel)and(mx>=hmenu[j].vmenu[i].x)and(my>=hmenu[j].vmenu[i].y)and
           (mx<=hmenu[j].vmenu[i].x+hmenu[j].vmenu[i].sizex)and
           (my<=hmenu[j].vmenu[i].y+hmenu[j].vmenu[i].sizey)
         then
          begin
            handlemouse:=hmenu[j].vmenu[i].code;
            tm:=hmenu[j].vmenu[i].code;
          end;
          active:=false;
          ssub:=false;
          show;
          ndlpntr^.handleevent(tm,'');
    end;
end;
destructor  tmenu.done;
begin
end;
{twnd------------------------------------------------------------------}
constructor twnd.init(px,py,sx,sy:integer;ttl:string;pntr:pHdl);
begin
  ndlpntr:=pntr;
  x:=px;y:=py;sizex:=sx;sizey:=sy;
  title:=ttl;
  first:=nil;last:=nil;
end;
procedure   twnd.show;
var pp:pgroup;
begin
  hidemouse;
  setfillstyle(1,0);
  bar(x,y,x+sizex,y+sizey);
  setfillstyle(1,14);
  bar(x+1,y+1,x+sizex-1,y+sizey-1);
  setfillstyle(1,4);
  bar(x+1,y+1,x+sizex-1,y+16);
  setcolor(0);
  line(x,y+16,x+sizex,y+16);
  setcolor(12);
  outtextxy(x+4,y+5,title);
  pp:=first;
  while pp<>nil do
    begin
      pp^.show;
      pp:=pp^.next;
    end;
  repaint:=false;
  showmouse;
end;
function    twnd.handlemouse:integer;
var pp:pgroup;
begin
  pp:=first;
  while pp<>nil do
    begin
      pp^.handlemouse;
      pp:=pp^.next;
    end;
end;
function twnd.handlekey(c:char):integer;
var pp:pgroup;
begin
  pp:=first;
  while pp<>nil do
    begin
      pp^.handlekey(c);
      pp:=pp^.next;
    end;
end;
procedure   twnd.insert(pp:pgroup);
begin
  pp^.owner:=@self;
  if ((first=nil)and(last=nil))then
    begin
      first:=pp;
      last:=pp;
    end else
    begin
      last^.next:=pp;
      last:=pp;
    end;
end;
destructor  twnd.done;
begin
end;
{tbutton-----------------------------------------------------------}
constructor tbutton.init(px,py,sx,sy:integer;ttl:string;i:integer;pntr:pHdl);
begin
  ndlpntr:=pntr;
  x:=px;y:=py;sizex:=sx;sizey:=sy;
  id:=i;
  active:=false;
  caption:=ttl;
  next:=nil;
end;
function tbutton.handlemouse:integer;
var tm:boolean;
begin
  if (mx>=(owner^.x+x))and(mx<=(owner^.x+x+sizex))and
  (my>=(owner^.y+y))and(my<=(owner^.y+y+sizey))
  then ndlpntr^.handleevent(id,'');
end;
function tbutton.handlekey(c:char):integer;
begin
end;
procedure tbutton.show;
begin
  setfillstyle(1,0);
  bar(owner^.x+x,owner^.y+y,owner^.x+x+sizex,owner^.y+y+sizey);
  if active
  then setfillstyle(1,15)
  else setfillstyle(1,12);
  bar(owner^.x+x+1,owner^.y+y+1,owner^.x+x+sizex-1,owner^.y+y+sizey-1);
  setcolor(4);
  outtextxy(owner^.x+x+4,owner^.y+y+4,caption);
end;
{tlabel-----------------------------------------------------------}
constructor tlabel.init(px,py:integer;ttl:string);
begin
  x:=px;y:=py;
  caption:=ttl;
  next:=nil;
end;
function tlabel.handlemouse:integer;
begin
end;
function tlabel.handlekey(c:char):integer;
begin
end;
procedure tlabel.show;
begin
  setcolor(4);
  outtextxy(owner^.x+x+4,owner^.y+y+2,caption);
end;
{tedit-----------------------------------------------------------}
constructor tedit.init(px,py,sx,sy:integer;ttl:string;i:integer;pntr:pHdl;dig:boolean);
begin
  digitonly:=dig;
  ndlpntr:=pntr;
  id:=i;
  x:=px;y:=py;sizex:=sx;sizey:=sy;
  text:=ttl;
  next:=nil;
end;
function tedit.handlemouse:integer;
var tm:boolean;
begin
  active:=
  (mx>=(owner^.x+x))and(mx<=(owner^.x+x+sizex))and
  (my>=(owner^.y+y))and(my<=(owner^.y+y+sizey));
  if active then show;
end;
function tedit.handlekey(c:char):integer;
begin
  if active then
    begin
      if c=#08 then delete(text,length(text),1) else
      if (digitonly=false)or((digitonly)and((length(text)<3)and(c in['0'..'9']))) then
        begin
          text:=text+c;
        end;
      hidemouse;
      show;
      showmouse;
      ndlpntr^.handleevent(id,text);
    end;
end;
procedure tedit.show;
begin
  setfillstyle(1,0);
  bar(owner^.x+x,owner^.y+y,owner^.x+x+sizex,owner^.y+y+sizey);
  setfillstyle(1,15);
  bar(owner^.x+x+1,owner^.y+y+1,owner^.x+x+sizex-1,owner^.y+y+sizey-1);
  setcolor(4);
  if active
  then outtextxy(owner^.x+x+4,owner^.y+y+2,text+'|')
  else outtextxy(owner^.x+x+4,owner^.y+y+2,text);
end;
{tmemo-----------------------------------------------------------------}
constructor tmemo.init(px,py,sx,sy:integer;pntr:pHdl);
begin
  ndlpntr:=pntr;
  x:=px;y:=py;sizex:=sx;sizey:=sy;
  next:=nil;
end;
function tmemo.handlemouse:integer;
var tm:boolean;
begin
  active:=
  (mx>=(owner^.x+x))and(mx<=(owner^.x+x+sizex))and
  (my>=(owner^.y+y))and(my<=(owner^.y+y+sizey));
  if active then show;
end;
function tmemo.handlekey(c:char):integer;
begin
  if active then
    begin
      if c=#08 then ;
      hidemouse;
      show;
      showmouse;
{      ndlpntr^.handleevent(id,text);}
    end;
end;
procedure tmemo.show;
var i:integer;
begin
  setfillstyle(1,0);
  bar(owner^.x+x,owner^.y+y,owner^.x+x+sizex,owner^.y+y+sizey);
  setfillstyle(1,15);
  bar(owner^.x+x+1,owner^.y+y+1,owner^.x+x+sizex-1,owner^.y+y+sizey-1);
  setcolor(0);
{  strcount:=127;}
{  for i:=1 to trncnt do
    outtextxy(owner^.x+x+10+((i-1) div 32)*140,
    owner^.y+y+10+(((i-1) mod 32)*10),
    inttostr(i)+'-'+inttostr(triangles[i].p1)+','+inttostr(triangles[i].p2)+','+inttostr(triangles[i].p3));}

end;
{tHdl-------------------------------------------------------------------}
procedure thdl.handleevent(h:integer;param:string);
begin
end;

end.
