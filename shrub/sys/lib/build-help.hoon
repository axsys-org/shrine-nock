
/*  noon-src  %hoon  /lib/hoon/hoon
/*  sept-src  %hoon  /sur/sept/hoon
/*  neo-src  %hoon   /lib/neo-lazy/hoon
|%
++  swel
  |=  [tap=(trap vase) gen=hoon]
  ^-  (trap vase)
  =/  gun  (~(mint ut p:$:tap) %noun gen)
  =>  [tap=tap gun=gun]
  |.  ~+
  =/  pro  q:$:tap
  [[%cell p.gun p:$:tap] [.*(pro q.gun) pro]]
++  slup
  |=  [sin=(trap vase) dex=(trap vase)]
  ^-  (trap vase)
  =>  [sin=sin dex=dex]
  |.  ~+
  =/  sun  $:sin
  =/  dux  $:dex
  [[%cell -.sun -.dux] [+.sun +.dux]]
::
+$  stud
  $@  @tas                                 ::  auth=urbit
  [i=iota t=pith]
++  rain
  |=  [=path src=@]
  ~|  path
  =>  [path=path src=src 4 ..zuse]
  ~>  %memo./pill/rain
  =+  vaz=vast
  (scan (trip src) (full (ifix [gay gay] tall:vaz(wer path))))
++  munt
  |=  =hoon
  =>  [hoon=hoon ..zuse]
  ~>  %memo./pill/munt
  (~(mint ut %noun) %noun hoon)
++  larval-pail
  !,  *hoon
  |%
  +$  pail  $+(larval-pail (pair stid *))
  --
++  thaw
  |=  pax=pith
  ^-  spec
  =;  el=(list spec)
    ?~  el  [%base %null]
    [%bccl i.el (snoc t.el %base %null)]
  %+  turn  pax
  |=  i=iota
  ^-  spec
  ?@  i  [%leaf %tas i]
  ;;(spec [%leaf i])

++  with-face  |=([face=@tas =vase] vase(p [%face face p.vase]))
++  surf
  =/  gen  (rain /sys/hoon noon-src)
  =/  reef=vase
    ~>  %slog.[0 leaf/"noon: acquiring reef"]
    ~<  %slog.[0 leaf/"noon: acquired reef"]
    =+  gun=(munt gen)
    [p.gun .*(~ q.gun)]
  |%
  ++  parse
    |=  [wer=path src=@t]
    =>  :*  reef=reef
            src=@t
            wer=path
            ..zuse
        ==
    :: ~>  %memo./noon/parse
    !<  hoon
    %+  slam
      %+  slap  reef
      !,(*hoon rain)
    !>([wer src])

  --


++  noon
  |=  =(map stud hoon)
  ^-  (trap vase)
  =/  gen
    ~>  %slog.[0 leaf/"noon: parsing reef"]
    (rain /sys/path noon-src)
  =/  reef=vase
    ::!>(..ride)
    ~>  %slog.[0 leaf/"noon: acquiring reef"]
    ~<  %slog.[0 leaf/"noon: acquired reef"]
    =+  gun=(munt gen)
    [p.gun .*(~ q.gun)]
  =/  raff=(trap vase)
    (swat *(trap vase) gen)
    ::::!>(..ride)
    ::~>  %slog.[0 leaf/"noon: acquiring reef"]
    ::~<  %slog.[0 leaf/"noon: acquired reef"]
    ::=+  gun=(munt gen)
    ::[p.gun .*(~ q.gun)]

  =|  sut=(trap vase)
  =.  sut  (swat sut gen)
  =/  piss
    |=  [wer=path src=@t]
    !<  hoon
    %+  slam
      %+  slap  reef
      !,  *hoon
      |=  [bon=path txt=@]
      ^-  hoon
      =+  vaz=vast
      ~|  bon
      (scan (trip txt) (full (ifix [gay gay] tall:vaz(wer bon))))
    !>([wer src])
  ~>  %slog.[0 leaf/"noon: parsing neo"]
  =/  neo-hoon  (piss /sys/neo neo-src)
  ~>  %slog.[0 leaf/"noon: parsing sept"]
  =/  sept-hoon  (piss /sys/sept sept-src)
  =/  pail-vase=(trap vase)
    (swat sut larval-pail)
  =/  sept=(trap vase)
    ~>  %slog.[0 leaf/"noon: compiling sept"]
    ~<  %slog.[0 leaf/"noon: compiled sept"]
    %+  swat  (slup pail-vase raff)
    sept-hoon
  =.  pail-vase
    ~>  %slog.[0 leaf/"noon: compiling pail"]
    ~<  %slog.[0 leaf/"noon: compiled pail"]
    %+  swat  sept
    (make-pail-core %pail-one map)
  =.  sept
    ~>  %slog.[0 leaf/"noon: installing pail"]
    ~<  %slog.[0 leaf/"noon: installed pail"]
    (swat (slup pail-vase raff) sept-hoon)
  =.  pail-vase
    ~>  %slog.[0 leaf/"noon: compiling pail"]
    ~<  %slog.[0 leaf/"noon: compiled pail"]
    %+  swat  sept
    (make-pail-core %pail-two map)

  =.  sept
    ~>  %slog.[0 leaf/"noon: installing pail"]
    ~<  %slog.[0 leaf/"noon: installed pail"]
    (swat (slup pail-vase raff) sept-hoon)
  ~>  %slog.[0 leaf/"noon: compiling neo"]
  ~<  %slog.[0 leaf/"noon: compiled sept"]
  ::=;  res=(each (trap vase) tang)
  ::  ?:  ?=(%& -.res)  p.res
  ::  (mean p.res)
  ::%-  mule  |.
  (swat sept neo-hoon)
++  sept-hoon   (rain /sys/sept sept-src)
++  non-empty  (noon ~)
++  non  (noon std-pro)
++  make-pail-core
  |=  [label=term studs=(map stud hoon)]
  :-  %ktwt
  :+  %brcn  ~
  %+  ~(put by *(map term tome))  %$
  ^-  tome
  :-  ~
  %+  ~(put by *(map term hoon))  %pail
  ^-  hoon
  =;  ls=(list spec)
    ?~  ls  !!
    [%ktcl %bcls label %bccn i.ls t.ls]
  %+  turn  ~(tap by studs)
  |=  [=stud =hoon]
  =/  std=spec
    ?^  stud  (thaw stud)
    [%leaf %tas stud]
  =/  ss=@tas  ?@(stud stud ?@(i.stud i.stud (scot i.stud)))
  [%bccl std [%bcls ss [%bcmc hoon]] ~]
::
::$%  [/std/pro/term term=@t]
::    [/std/pro/date date=@da]




::
++  std-pro
  |^
  ^-  (map stud hoon)
  %-  ~(gas by *(map stud hoon))
  :~  (like %noun)
      (like %vase)
      (like %wain)
      (like %cord)
      (like %tang)
      (like %hoon-src)
      (like %hoon)
      (like %tang)
      (like %atom)
      :: (like %clue)
      (like %crew)
      (like %fans)
      :: (setlike %move)
      (sept-like %duct)
      :: (sept-like %card)
       (sept-like %dish)
      (sept-like %pith)
      ::(sept-like %ewer)
  ==
  ++  like
    |=  =term
    ^-  (pair stud hoon)
    :-  term
    [%ktcl %like ~[term] ~]
  ++  sept-like
    |=  =term
    ^-  (pair stud hoon)
    :-  term
    [%tsgl [%wing ~[term]] [%wing ~[%t]]]
  --
+$  vial  (pair stud *)
+$  tale  (map pith vial)
+$  leaf  (pair pith tale)
+$  germ
  [%germ kern=(list) items=(list)]
++  make-germ
  ^-  germ
  =/  ker  non
  =/  nok  !.
    =>  *[ver=(trap vase) ~]
    !=  q:$:ver
  =/  kern=(list)
    [nok ker ~]
  =|  items=(list)
  germ/[kern items]
--

