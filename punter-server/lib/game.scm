(define-module (lib game)
  #:use-module (ext-lib pipe)  
  #:use-module (lib game-data)
  #:use-module (lib log)
  #:use-module (srfi srfi-1)
  #:export (*game*
            *game-state*
            connected-rivers
            game-score
            declare-futures
            apply-claim
            apply-pass
            apply-splurge
            apply-option))

(define *game* (make-fluid #nil))

(define *game-state* (make-fluid #nil))

(define (connected-rivers site-id)
  (let* ((cur-game (fluid-ref *game*))
         (cur-map (game-game-map cur-game)))
    (->> (game-map-rivers cur-map)
         (filter (lambda (river)
                   (or (eq? site-id (river-source river))
                       (eq? site-id (river-target river))))))))

(define (node-site node)
  (car node))

(define (node-weight node)
  (cadr node))

(define (node-reachable? node)
  (caddr node))

(define (node-finished? node)
  (cadddr node))

(define (node-less? n1 n2)
  (let ((ns1 (node-site n1))
        (ns2 (node-site n2))
        (nf1 (node-finished? n1))
        (nf2 (node-finished? n2)))
    (cond
     ((= ns1 ns2) (cond
                   ((and (not nf1) nf2) #t)
                   ((and nf1 (not nf2)) #f)
                   (#t #t)))
     (#t (< ns1 ns2)))
    (< (node-weight n1)
             (node-weight n2))))

(define (node-eq? n1 n2)
  (= (node-site n1)
     (node-site n2)))

(define (set-node-weight node weight)
  `(,(node-site node)
    ,weight
    ,(node-reachable? node)
    ,(node-finished? node)))

(define (set-node-reachable node reachable)
  `(,(node-site node)
    ,(node-weight node)
    ,reachable
    ,(node-finished? node)))

(define (set-node-finished node finished)
  `(,(node-site node)
    ,(node-weight node)
    ,(node-reachable? node)
    ,finished))

(define (punter-game-score punter dijkstra-maps)
  (->> dijkstra-maps
      (map cdr)
      (apply append)
      (map (lambda (node)
             (if (node-reachable? node)
                 (* (node-weight node) (node-weight node))
                 0)))
      (apply +)))

(define (punter-futures-score punter dijkstra-maps futures)
  (if futures
      (->> futures
           (map (lambda (future)
                  (let* ((fsource (future-source future))
                         (ftarget (future-target future))
                         (future-dijkstra (assq-ref dijkstra-maps fsource))
                         (target-node (assq ftarget future-dijkstra))
                         (score-value (* (node-weight target-node)
                                         (node-weight target-node)
                                         (node-weight target-node))))
                    (if (node-reachable? target-node)
                        score-value
                        (- score-value)))))
           (apply +))
      0))

(define (river->node djmap claims options punter node river)
  (let* ((current-weight (+ (node-weight node) 1))
         (opp-site (if (eq? (node-site node) (river-source river))
                       (river-target river)
                       (river-source river)))
         (opp-reachable (and (node-reachable? node)
                             (or (eq? punter
                                      (hash-ref claims
                                                (sort (list (river-source river)
                                                            (river-target river)) <) #f))
                                 (eq? punter
                                      (hash-ref options
                                                (sort (list (river-source river)
                                                            (river-target river)) <) #f)))))
         (opp-node (or (find
                        (lambda (node)
                          (eq? opp-site (node-site node)))
                        djmap)
                       `(,opp-site
                         ,current-weight
                         ,opp-reachable
                         #f)))
         (new-reachable (or opp-reachable (node-reachable? opp-node))))
    (-> opp-node
        (set-node-weight (min current-weight (node-weight opp-node)))
        (set-node-reachable new-reachable)
        (set-node-finished (if (and (node-finished? opp-node) new-reachable (not (node-reachable? opp-node)))
                               #f
                               (node-finished? opp-node)))
        )))

(define (compute-dijkstra-map djmap claims options punter)
  (let* ((min-value-node (-> djmap
                             (->> (filter (lambda (node)
                                            (not (node-finished? node)))))
                             (sort node-less?)
                             (->> (find (lambda (node)
                                          #t))))))
    (if min-value-node
        (let* ((upd-nodes (cons
                           (set-node-finished min-value-node #t)
                           (map
                            (lambda (river)
                              (river->node djmap claims options punter min-value-node river))
                            (connected-rivers (node-site min-value-node))))))
          (compute-dijkstra-map
           (append
            upd-nodes
            (remove (lambda (node-in)
                      (find (lambda (node)
                              (node-eq? node-in node))
                            upd-nodes))
                    djmap))
           claims
           options
           punter))
        djmap)))

(define (compute-dijkstra-maps mines claims options punter)
  (map
   (lambda (mine-id)
     (cons mine-id (compute-dijkstra-map `((,mine-id 0 #t #f)) claims options punter)))
   mines))

(define (game-score)
  (let* ((cur-game (fluid-ref *game*))
         (cur-game-state (fluid-ref *game-state*))
         (all-punters (punters-list (game-punters-count cur-game)))
         (all-mines (game-map-mines (game-game-map cur-game)))
         (claims (game-state-claims cur-game-state))
         (options (game-state-options cur-game-state)))
    (map
     (lambda (punter)
       (let ((punter-dijkstra-maps (compute-dijkstra-maps all-mines claims options punter))
             (futures (hash-ref (game-state-futures cur-game-state) punter)))
         `(,punter . ,(+ (punter-game-score punter punter-dijkstra-maps)
                         (punter-futures-score punter punter-dijkstra-maps futures)))))
     all-punters)))

(define (init-game)
  (let ((cur-game (fluid-ref *game*))
        (cur-game-state (fluid-ref *game-state*)))
    #f))

(define (declare-futures punter pfutures)
  (let* ((cur-game-state (fluid-ref *game-state*))
         (futures (game-state-futures cur-game-state)))
    (hash-set! futures punter pfutures)))

(define (apply-claim punter river)
  (let* ((cur-game (fluid-ref *game*))
         (cur-game-state (fluid-ref *game-state*))
         (claims (game-state-claims cur-game-state))
         (current-moves (game-state-moves cur-game-state))
         (rsource (river-source river))
         (rtarget (river-target river))
         (source (min rsource rtarget))
         (target (max rsource rtarget))
         (river-def (list source target)))
    (if (hash-ref claims river-def)
        (begin
          (add-game-state-move! cur-game-state (pass-move punter)))
        (begin
          (hash-set! claims river-def punter)
          (add-game-state-move! cur-game-state (claim-move punter rsource rtarget))))))

(define (apply-pass punter)
  (let* ((cur-game-state (fluid-ref *game-state*)))
    (add-game-state-move! cur-game-state (pass-move punter))))

(define (apply-splurge punter route)
  (let* ((cur-game (fluid-ref *game*))
         (cur-game-state (fluid-ref *game-state*))
         (claims (game-state-claims cur-game-state))
         (options (game-state-options cur-game-state)))
    (-> (map
         (lambda (site1 site2)
           (list (min site1 site2) (max site1 site2)))
         (take route (- (length route) 1))
         (cdr route))
        (->> (map (lambda (river-def)
                    (if (hash-ref claims river-def)
                        (hash-set! options river-def punter)
                        (hash-set! claims river-def punter))))))
    (add-game-state-move! cur-game-state (splurge-move punter route))))

(define (apply-option punter river)
  (let* ((cur-game (fluid-ref *game*))
         (cur-game-state (fluid-ref *game-state*))
         (claims (game-state-claims cur-game-state))
         (options (game-state-options cur-game-state))
         (rsource (river-source river))
         (rtarget (river-target river))
         (source (min rsource rtarget))
         (target (max rsource rtarget))
         (river-def (list source target)))
    (if (and (hash-ref claims river-def)
             (not (hash-ref options river-def)))
        (begin
          (hash-set! options river-def punter)
          (add-game-state-move! cur-game-state (option-move punter rsource rtarget)))
        (begin
          (add-game-state-move! cur-game-state (pass-move punter))))))
