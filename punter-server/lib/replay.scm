(define-module (lib replay)
  #:use-module (ext-lib pipe)
  #:use-module (ice-9 format)
  #:use-module (ice-9 popen)
  #:use-module (ice-9 rdelim)
  #:use-module (json)
  #:use-module (lib game)
  #:use-module (lib game-data)
  #:use-module (lib loader)
  #:use-module (lib log)
  #:use-module (srfi srfi-1)
  #:use-module (srfi srfi-9)
  #:export (generate-replay))

(define _SERVER_TO_PUNTER_ "S -> P")

(define _PUNTER_TO_SERVER_ "P -> S")

(define-record-type replay-context
  (make-replay-context my-id game game-state)
  replay-context?
  (my-id rc-my-id set-rc-my-id!)
  (game rc-game set-rc-game!)
  (game-state rc-game-state set-rc-game-state!)
  (f-counter rc-fcounter set-rc-fcounter!))

(define (site-by-id sites id)
  (find
   (lambda (site)
     (eq? id (site-id site)))
   sites))

(define (site->gnuplot site)
  (format #f "set label at ~a,~a \"~a\" font \"DejaVuSans,30\" front point pointtype 7 pointsize 2 linecolor rgb 'orange'"
          (site-x site)
          (site-y site)
          (site-id site)))

(define (mine->gnuplot site)
  (format #f "set label at ~a,~a \"\" font \"DejaVuSans,30\" front point pointtype 7 pointsize 5 linecolor rgb 'cyan'"
          (site-x site)
          (site-y site)))

(define (river->gnuplot river map-sites)
  (let ((site-source (site-by-id map-sites (river-source river)))
        (site-target (site-by-id map-sites (river-target river))))
    (format #f "set arrow from ~a,~a to ~a,~a nohead linecolor rgb 'gray' linewidth 3"
            (site-x site-source)
            (site-y site-source)
            (site-x site-target)
            (site-y site-target))))

(define (my-river->gnuplot river map-sites)
  (let ((site-source (site-by-id map-sites (river-source river)))
        (site-target (site-by-id map-sites (river-target river))))
    (format #f "set arrow from ~a,~a to ~a,~a nohead linecolor rgb 'green' linewidth 5"
            (site-x site-source)
            (site-y site-source)
            (site-x site-target)
            (site-y site-target))))

(define (claimed-river->gnuplot river map-sites)
  (let ((site-source (site-by-id map-sites (river-source river)))
        (site-target (site-by-id map-sites (river-target river))))
    (format #f "set arrow from ~a,~a to ~a,~a nohead linecolor rgb 'red' linewidth 5"
            (site-x site-source)
            (site-y site-source)
            (site-x site-target)
            (site-y site-target))))

(define (gp-data-as-str gp-list)
  (-> gp-list
      (->> (map (lambda (strs) (string-join strs "\n"))))
      (string-join "\n")))

(define (game-map->gnuplot game-map)
  (let* ((map-sites (game-map-sites game-map))
         (sites-strs (map site->gnuplot map-sites))
         (mines-strs (map
                      (lambda (mine)
                        (let ((mine-site (site-by-id map-sites mine)))
                          (mine->gnuplot mine-site)))
                      (game-map-mines game-map)))
         (rivers-str (map (lambda (river)
                            (river->gnuplot river map-sites))
                          (game-map-rivers game-map))))
    (list sites-strs
          mines-strs
          rivers-str)))

(define (game-state->gnuplot my-id game-map game-state)
  (let* ((map-sites (game-map-sites game-map))
         (claims-strs (-> (game-state-claims game-state)
                          (->> (hash-map->list cons))
                          (->> (map (lambda (claim)
                                      (let* ((river-def (car claim))
                                             (pid (cdr claim))
                                             (river (apply make-river river-def)))
                                        (if (eq? my-id pid)
                                            (my-river->gnuplot river map-sites)
                                            (claimed-river->gnuplot river map-sites)))))))))
    (list claims-strs)))

(define (gp-prelude rctx)
  (list (list "set terminal png size 640, 640"
              "set output '/tmp/tri.png'")))

(define (gp-footer rctx)
  (let* ((map-sites (game-map-sites (game-game-map (rc-game rctx))))
         (map-xs (map site-x map-sites))
         (map-ys (map site-y map-sites))
         (min-x (apply min map-xs))
         (max-x (apply max map-xs))
         (min-y (apply min map-ys))
         (max-y (apply max map-ys))
         (width (- max-x min-x))
         (height (- max-y min-y))
         (x-border (* 0.2 width))
         (y-border (* 0.2 height))
         (border (min x-border y-border)))
    (list (list "unset key"
                "unset tics"
                "unset border"
                (format #f"plot [~a:~a] [~a:~a] 1/0 linewidth 0"
                        (- min-x border)
                        (+ max-x border)
                        (- min-y border)
                        (+ max-y border))))))

(define (to->gnuplot rctx)
  (let* ((my-id (rc-my-id rctx))
         (game (rc-game rctx))
         (game-state (rc-game-state rctx))
         (game-map-strs (game-map->gnuplot (game-game-map game)))
         (game-state-strs (game-state->gnuplot my-id (game-game-map game) game-state))
         (gp-footer-strs (gp-footer rctx)))
    (-> (append game-map-strs
                game-state-strs
                gp-footer-strs)
        (gp-data-as-str))))

(define (gp-exec plot-prog frame-number)
  (let ((w-port (open-output-pipe "gnuplot")))
    (format w-port "set terminal png size 640, 640~&")
    (format w-port "set output '/tmp/tri~5,,,'0@a.png~&" frame-number)
    (display plot-prog w-port)
    (when (not (eqv? 0 (status:exit-val (close-pipe w-port))))
      (throw 'gnuplot-error))))

(define (setup-game rctx jsval)
  (let* ((my-id (hash-ref jsval "punter"))
         (punters-count (hash-ref jsval "punters"))
         (settings (hash-ref jsval "settings"))
         (game-map (transform->game-map (hash-ref jsval "map")))
         (game (make-game punters-count game-map))
         (game-state (make-game-state punters-count)))
    (set-rc-fcounter! rctx 0)
    (set-rc-my-id! rctx my-id)
    (set-rc-game! rctx game)
    (set-rc-game-state! rctx game-state)))

(define (apply-claim-move move)
  (apply-claim (hash-ref move "punter")
               (make-river (hash-ref move "source")
                           (hash-ref move "target"))))

(define (apply-pass-move move)
  #nil)

(define (apply-move move)
  (cond
   ((hash-ref move "claim") (apply-claim-move (hash-ref move "claim")))
   ((hash-ref move "pass")  (apply-pass-move  (hash-ref move "pass")))
   (#t (throw 'illegal-state))))

(define (apply-opponent-moves rctx jsval)
  (let* ((moves (-> jsval
                    (hash-ref "move")
                    (hash-ref "moves")))
         (game (rc-game rctx))
         (game-state (rc-game-state rctx))
         (p-count (game-punters-count game)))
    (with-fluids ((*game* game)
                  (*game-state* game-state))
      (map
       (lambda (move)
         (apply-move move))
       (filter
        (lambda (move)
          (let ((cur-punter (hash-ref (cdar (hash-map->list cons move)) "punter")))
            (not (eq? cur-punter (rc-my-id rctx)))))
        moves)))))

(define (handle-s->p rctx jsval)
  (cond
   ((hash-ref jsval "map") (setup-game rctx jsval))
   ((hash-ref jsval "move") (apply-opponent-moves rctx jsval))))

(define (handle-me rctx jsval)
  #nil)

(define (handle-ready rctx jsval)
  #nil)

(define (handle-my-moves rctx jsval)
  (let* ((current-fc (rc-fcounter rctx))
         (next-fc (+ current-fc 1))
         (next-next-fc (+ next-fc 1))
         (game (rc-game rctx))
         (game-state (rc-game-state rctx)))
    (gp-exec (to->gnuplot rctx) current-fc)
    (set-rc-fcounter! rctx next-fc)
    (with-fluids ((*game* game)
                  (*game-state* game-state))
      (cond
       ((hash-ref jsval "claim") (apply-claim-move (hash-ref jsval "claim")))
       ((hash-ref jsval "pass")  (apply-pass-move  (hash-ref jsval "pass")))))
    (gp-exec (to->gnuplot rctx) next-fc)
    (set-rc-fcounter! rctx next-next-fc)))

(define (handle-p->s rctx jsval)
  (cond
   ((hash-ref jsval "me")    (handle-me rctx jsval))
   ((hash-ref jsval "ready") (handle-ready rctx jsval))
   (#t (handle-my-moves rctx jsval))))

(define (handle-protocol-message rctx data-type data)
  (let* ((colon-sep (string-index data #\:))
         (msg-len (string->number (substring data 0 colon-sep)))
         (msg (substring data (+ colon-sep 1)))
         (json-data (json-string->scm msg)))
    (cond
     ((string=? data-type _SERVER_TO_PUNTER_) (handle-s->p rctx json-data))
     ((string=? data-type _PUNTER_TO_SERVER_) (handle-p->s rctx json-data)))))

(define (handle-unknown-message rctx data)
  ;; ignore now
  #nil)

(define (handle-line rctx line)
  (when (string-contains line "DEBUG:lambda_punter::client:")
    (let* ((actual-line (substring line 29))
           (info-sep (string-index actual-line #\|)))
      (when info-sep
        (let ((info-type (string-trim-both (substring actual-line 0 info-sep)))
              (info-data (string-trim-both (substring actual-line (+ info-sep 1)))))
          (cond
           ((or (string=? info-type _SERVER_TO_PUNTER_)
                (string=? info-type _PUNTER_TO_SERVER_)) (handle-protocol-message rctx info-type info-data))
           (#t (handle-unknown-message rctx info-data))))))))

(define (generate-replay replay-file output-dir)
  (let ((rctx (make-replay-context -1 #nil #nil)))
    (with-input-from-file replay-file
      (lambda ()
        (let loop ((line (read-line)))
          (when (not (eof-object? line))
            (when (not (string=? "" line))
              (handle-line rctx line))
            (loop (read-line))))))))
