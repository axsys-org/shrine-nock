=>
  |%
  ++  make-err
    |=  [=tale:t err=wain]
    ^+  tale
    (~(put by tale) vase:slot:t [%wain err])
  ++  make-faced-slot
    |=  =term
    ^-  pith:t
    #/sys/slot/face/[term]
  ++  make-sut
    |=  [=bowl:n =tale:t]
    =/  std  |.  prelude:n
    %+  roll  ~(tap by deps.bowl)
    |=  [[slot=road:t =pith:t =epic:t] out=_std]
    ?.  ?=([%sys %slot %face face=@ ~] slot)
      ~|  weird-slot/slot
      !!
    =/  =dish:t
      =/  p  (need fil.epic)
      ?:  ?=(%dish -.p)
      +.p
    (slup:ford (with-face:ford face.slot 
    
  ++  build
    |=  =tale:t
    ^+  tale
    ?~  hon=(~(get by tale) ast:slot:t)
      (make-err tale 'No ast file' ~)
    ?.  ?=(%hoon -.u.src) 
      (make-err tale 'AST slot not hoon' ~)
    
    =/  lon=(each loon:ford hair)  
      (slap 
    ?:  ?=(%| -.lon)
      (make-err (crip "Syntax err: line {<q.p.lon>} col {<p.p.lon>}") ~)
    =/  =crew:n
      %-  ~(gas by *crew:n)
      %-  welp
        %+  murn  pro.pile.p.lon
        |=  [face=term =stud:t]
        ^-  (unit [slot:t pith:t])
        ?@  stud  ~
        `[(make-faced-slot face) (stud-to-pith stud %pro)]
      %+  turn  vaz.pile.p.lon
      |=  [face=term =pith:t]
      ^-  [slot:t pith:t]
      [(make-faced-slot face) pith]
    %-  ~(gas by tale)
    :~  [ast:slot:t %hoon hoon.p.lon]
        [crew:slot:t %crew crew]
    ==
  --
^-  form:n
|_  [=bowl:n =saga:t]
::  handle initial
++  init
  |=  =tale:t
  ^-  (quip card:n tale:t)
  `(build here.bowl tale)
::  handle change to state
++  talk
  |=  =tale:t
  ^-  (quip card:n tale:t)
  `(build here.bowl (~(uni by q.saga) tale))
::  handle change to children
++  take
  |=  =gift
  ^-  (quip card:n tale:t)
  `q.saga
::  handle death of dependency
++  dead
  |=  =slot:t
  ^-  (quip card:n tale:t)
  `q.saga
::  hear: handle depnendency change 
++  hear
  |=  =rely
  ^-  (quip card:n tale:t)
  `q.saga
::  goof: handle possible error
++  goof
  |=  [c=card:n err=(unit (list tank))]
  ^-  (quip card:n tale:t)
  `q.saga
--



